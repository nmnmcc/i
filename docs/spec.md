# The i Language


This document is the definition of the i programming language. A compiler or a program that does not keep to it is wrong. The present compiler does not yet do everything in this document.

This document is complete in itself. It defines:

- the source language (§1 to §8);
- effects, and the library modules `i/fail`, `i/control`, `i/resource`, `i/assertion`, `i/io`, `i/key`, `i/task`, `i/random`, `i/request`, `i/clock`, `i/fs`, `i/process`, `i/net`, `i/tls`, and `i/stdio` (§9);
- the library module `i/core`, which has the operations that are not written in i (§13);

- macros, the Lisp surface, and the library module `i/syntax` (§14);
- the library module `i/float_text`, the text form of a Float written in i (§11.8);
- packages, versions, and the files `i.toml` and `i.lock` (§2.1, §2.6 to §2.8);
- what programs do when they run, and the text form of values (§11);
- the binary interface between a compiled module and its host (§10 and Appendix B).

Every compiler must give the library modules above. The target modules `i/target`, `i/js`, and `i/wasi` have the contracts of §9.7, §10.1 and §B.7.1. A program may not define a module with any of these library paths. Other library modules, command-line behavior not specified in §2.6, and the way test results are shown are not part of the language.


i is small, and every part of it is what it looks like. A thing that looks like a function is a normal function, and a chain of branches is a function too (§6.12). A binding is a binding wherever it is written, and a parameter group is a pattern matched as a binding matches. Where two forms look the same, they have the same rules, with this exception, where the place of a form decides its rules:

- `x: T` in a parameter item is a type annotation when its branch is the only branch of its shape group, has no guard, and the item has no alternatives, and a test when not (§7.1). So with `v: Int | String`, `v |> (x: Int) { x }` is a compile error, but `v |> (x: Int) { x } | (_) { 0 }` is correct.


## 0. How to read this document

The grammar is in EBNF: `|` is a choice, `[ ]` may be there or not, `{ }` may come zero or more times, and text in quotes is written as it is.


Inside a type in the rules and in the prose, a capital letter, alone or with a digit, such as `A`, `T`, `E`, `N`, or `P1`, stands for any type, and a lowercase letter, alone or with a digit, such as `a`, `r`, or `b1`, stands for a type variable. A letter followed by an index, `i`, `j`, `k`, `m`, or `n`, as in `Ti`, `Qi`, `ai`, `an`, or `bm`, is that letter with that index, and stands for the same kind of thing as the letter alone. Outside a type, the letters keep the sense that the text gives them: in §4.8, for example, `C` is a constructor, `e` an expression, and `r` an expression or a name. This is notation of this document, not i syntax: in i code, a type variable is an `UpName` that is declared as a type parameter (§4.7), as in `<A>(A) -> A`.


These words have a fixed sense in this document:

- **Compile error**: the compiler does not take the program and makes no output.
- **Stop**: the program ends at once while it is running. The host gets a message (§11.7). In a test, a stop makes that test fail, and the other tests go on.

- **Error**: a value raised through the service `Fail` in place of a normal value (§9.3). An error is not a compile error.

- **Host**: the system that runs the compiled Wasm module, for example a web browser or Wasmtime.
- **Type constructor**: a type name with a fixed number of parameters. `List` has one parameter; `Int` has none.
- **Program source**: all modules but the library modules (§2.1). Built-in names (§12) are not in the program source.
- **Not fixed**: this document does not say which result is given. A compiler may give any result that keeps to the stated rules, but the same compiled module gives the same result every time.

## 1. Words and symbols

### 1.1 Source files

A source file is UTF-8 text, and its name ends in `.i` (the normal surface) or `.is` (the Lisp surface, §14). `\r\n` and `\r` are the same as `\n`.

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


i has no keywords. Every word is a `name`, an `UpName`, or a discard (§1.3), and any `name` may be used as a name. The forms that other languages make with keywords are made with brackets and symbols: imports and exports are groups at the start of a file (§2.2), matching is a call of a chain of functions (§6.12), and a guard is written with `?` inside a parameter group.


Some forms are macro calls (§14): an attribute that is not built in, such as `@m` or `@m(args)` before an item (§2.2); a top-level item that is a call, `m(args)` or `a.m(args)` (§2.2); a declaration with neither `@external` nor `@js`, which goes with a typed macro binding of the same name (§14.8); and, in a type, a lowercase name or `a.m` followed by `(` (§3). The parser reads each of them in full without knowing which names are macros. Each of them is a compile error when its name is not a macro there.


### 1.5 Symbols

```
=   :   ,   .   ..   ->   <-   ?   |   @
(   )   [   ]   {   }   <<   >>
+   -   *   /   %   <   <=   >   >=   ==   !=   &&   ||   <>   &
+.  -.  *.  /.      <.  <=.  >.  >=.
```

Source text is cut into tokens from left to right. At every point, the longest token that matches is taken: `<=.` is one token, and `<<` is one token. `<<` is matched before `<-`, so `<<-1` is `<<` and `-1`; `x<-1` is `x`, `<-` and `1`, and a comparison with a negative number is written `x < -1`. `!` alone and `^` are not tokens. `<` and `>` also enclose type parameters (§4.7). Spaces (U+0020 and tab), line ends, and comments come between tokens, and some line ends are breaks (§1.7). After `.`, digits are read as a `decimal`, never as a float: `t.0.1` is `(t.0).1`. A token may not be followed directly by a letter, a digit, or `_` that would continue it as a word or a number: `12px`, `0xG`, `1.0E5`, and `fooBar` are compile errors.


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

- In an expression, `"a\(e)b"` interpolates an expression `e`, including calls, blocks and nested string literals. Every interpolated expression must have type `String`; there is no implicit conversion. Before the core tree, interpolation becomes `<>` of its parts from the left, leaving out empty text parts. `"a\(x)b"` is `"a" <> x <> "b"`; a sole expression keeps an empty part: `"\(x)"` is `"" <> x`. Interpolation is forbidden in labels, attribute arguments, import paths, literal types and patterns. The normal formatter retains interpolation as written; the Lisp surface has no interpolation and uses `<>`.

- A multi-line string begins with `"""` immediately followed by a line break. Its closing `"""` is on a line preceded only by spaces or tabs. That exact indentation prefix is removed from every nonblank content line; a line with less or different indentation is a compile error. Blank lines with less indentation are accepted and made empty; indentation beyond the closing prefix remains part of a blank line. The opening line break and the line break before the closing line are not part of the value. Escapes and interpolation work as in ordinary strings. A backslash immediately before a physical line break joins that line to the next after indentation is removed.
- A raw string surrounds its quotes with one or more matching `#`, as in `#"text"#`, `##"text"##` or `#"""` … `"""#`. Backslashes are literal unless followed by exactly the opening number of hashes. With one hash, `\#n` is a line feed and `\#(e)` interpolates, while `\n` and `\(e)` remain literal text. Raw multi-line strings use the same indentation and delimiter-line rules; `\#` at line end joins lines in a one-hash raw string. Closing quotes consume the matching number of hashes; too few do not close the string.


### 1.7 Lines

A line end is a **break** when both sides of it allow one:

- the last token before it **can end** an expression: a name, an `UpName`, a discard, a literal, `)`, `]`, `}`, or `>>`; and
- the first token after it **can begin** a statement: a name, an `UpName`, a discard, a literal, `(`, `[`, `{`, `<<`, `-`, `-.`, `..`, or `@`, or `<` when it begins type parameters (below).

Comments are not tokens, so the tokens on the two sides of a comment are what count. Every line end that is not a break is only a space.

A break ends something. What it ends depends on the brackets it is directly inside:

- Directly inside the `{ }` of a block, and at the top level of a file, a break ends a statement or an item. Attributes are part of the item after them, so a break after an attribute does not end the item.
- Directly inside `( )` (data, data types, data patterns, parameter groups, service patterns and needs), `[ ]`, or `<< >>`, a break is the same as a `,`. A `,` at the end of a line may be left out.
- Directly inside a `{ }` that groups a type (§3), a break is a compile error.

With no break, as on one line, a statement is the longest expression that may be read from its start, and a `,` is needed between parts. So statements may share a line: `b |> (True) { 1 } | (False) { 0 }`.

But a statement that starts with `(`, `-`, or `-.` begins a new line, because the expression before it on the same line goes on with it. In `{ a = f  (b, c) = g … }`, `f(b, c)` is read as a call, so the statement is the expression `f(b, c)` followed by a stray `=`, a compile error. With a constructor, the one binding may be correct and mean something else: `{ mk = Pair  (b, c) = g … }` is the binding `mk = Pair(b, c) = g`, which takes `g` apart, and it does not bind `mk` to `Pair`.

From this rule:

- When `(` is on the same line as the expression before it, it is a call (§6.6). When `(` is the first thing on a line, it begins a new statement or part.
- The `{` of a function body (§6.8) is on the same line as the `)` or the type before it.
- A line that ends in an operator, `,`, `=`, `:`, `->`, `<-`, `(`, `[`, `{`, or `<<` goes on in the next line. A line that begins with `.`, `|`, `?`, `->`, `<-`, or a binary operator other than `-` and `-.` goes on from the line before. So a branch of a chain that begins a line begins it with `|` (§6.12), and a provide may begin a line with `<-`.
- `a` and then `- b` on the next line are two statements. To write `a - b` on two lines, put `-` at the end of the first line.
- A line whose first tokens are `<`, one or more `UpName`s with `,` between them, `>` and `(` begins a new statement: a function with type parameters (§6.8). To go on with a comparison in that form on the next line, put the operator at the end of the line before.

One rule does not depend on lines: in the `->` part of a function, after the `)` of its parameters, a `{` right after `->`, `|`, `&`, or `-` is part of the type (§3). The first other `{` that is not inside brackets begins the body.


`i fmt` formats `.i` and `.is` source files in their own surface, including
both extensions by default in `src/`, `test/` and given directories;
`--check` reports files that would change. Both surfaces use 80 columns. The
normal surface indents with one tab per level. In the normal surface, the
**call depth** of an expression is the number of nested function calls on the
deepest path through call arguments: a call whose callee is an `UpName`
constructor and data literals do not count, field access and pipes do not add
depth, and a function literal argument starts again at 0 for its body. A call
whose call depth is greater than 2 is never laid out on one line: its
arguments go one per line, indented, with the closing `)` on its own line (the
layout of a call that does not fit the width), and the rule applies again to
each argument; a block whose one statement is broken so is over several
lines. The Lisp surface is laid out
by Lisp convention (cljfmt indentation): a datum that fits stays on one
line; closing brackets never begin a line and are gathered at the end of the
form's last line; indentation is with spaces. A broken block-like form (`=`,
`->`, `|`, `.`, `@`, `:`, `!`) keeps its head and distinguished first parts
on the head line and indents its body two spaces; another call either keeps
its first argument on the head line and aligns the others under it, or puts
its arguments under the head, one space in; `{`, `[` and `<<` align their
parts under the first. Reader prefixes stay attached, strings are not split,
comments keep their place (a line comment before closing brackets moves them
to the next line), and at most one existing blank line separates top-level
items. A single indivisible atom, string or comment may exceed the width.
`i fmt --to is` uses that same layout.

## 2. Modules

### 2.1 Packages and modules

A **package** is a directory with an `i.toml` file in it (§2.6). Every `.i` or `.is` file under its `src/` or `test/` directory is a **module** of the package.

The **path** of a module is the `name` of its package (§2.6), then `/`, then its path under `src/` or `test/`, without `.i` or `.is`, with `/` between the parts. In the package named `app`, the path of `src/users/admin.i` is `app/users/admin`. Every part of a path must be a `name`. Two modules of one package with the same path are a compile error.

Modules in `test/` are checked when their package is checked (§2.6), and compiled as test entries when its tests are run. They may import modules in `src/`, but not the other way.

A test module may be imported only by test modules of the same package. Its exports alone do not make its bindings used (§2.4.1).

In a module, the first part of a module path says which package the module is in. It must be one of these:

- `i`, for the **library modules** (§9, §10, §13, §14, §15, §B.7.1) and kept paths named at the start of this document. How it gives them is not fixed: the operations that cannot be written in i may be built into the compiler, and they are not host imports (§10.1);
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


A file has at most one imports group and at most one exports group. They come first, the imports before the exports. Neither may be empty: a file with nothing to import or export leaves the group out. Attributes may not come before the imports or the exports: a group in `( )` with an attribute before it is an item. A group in `( )` that is followed by `=` is a binding with a data pattern, as in `(even, odd) = …`. Any other group in `( )` at the top level, with or without attributes before it, is a compile error.



An item that is a `name`, then `:` and a type, with no `=` after the type, is a declaration, whatever its attributes. Any other item that starts with an `UpName` is a type definition (§4.3). An item that is a call, `m(args)` or `a.m(args)`, with no `=` or `:` after its `)`, is a `MacroItem`. Every other item is a binding (§2.5).


A `MacroItem` is a macro call in the position of an item (§14.5). When its name is not a macro, it is a compile error.

A declaration requires `@external` or `@js` (§10.1), or is the signature of a typed macro, with `@macro` in a `.is` file and a binding of the same name (§14.8). Any other declaration is a compile error, unless a macro before it replaces it.


In one module, every top-level name is defined one time only. Two bindings may not bind the same name, and two type definitions may not define the same type. Each labelled type defines one constructor with its name (§4.3). A constructor and a top-level binding may not have the same name either, because they are in one space of names (§2.3).


These are the attributes. An attribute whose name is one of them, with no module name before it, is that attribute. Its arguments must be string literals, given by position, with no labels,. Any other argument, such as `@external("env" <> "", "f")`, `@external(module: "env", "f")`, is a compile error. Any other attribute, such as `@m`, `@m(args)`, or `@a.m(args)`, is a macro before an item (§14.5); `@macro` marks a macro definition in a `.is` file (§14.5). An attribute in another place, or one attribute given two times on one item, is a compile error.

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


Header groups are not data. A colon does not introduce an import alias.


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


Module names, value names, and constructor names are in one space of names. So a module name may not be the same as a top-level name or another imported name of the module, and a local name may cover a module name. In `a.b`, when `a` is a value, `.b` reads a key (§6.7); when `a` is a module, `a.b` is a member of the module. Type names are in a different space, so a type and a value may have the same name.



### 2.4 Exports

```
ExportItem = name | UpName [ "(" ".." ")" ] .
```

- A top-level name that is not in the exports may only be used in its module.
- For a labelled type `T` (§4.3), `T` exports the type but not its constructor: other modules may use the type, but may not make its values, take them apart, read their keys, or spread them. `T(..)` exports the type and its constructor.
- For a name `A` (§4.3), `A` exports the name. `A(..)` exports the name and every member of its expansion with `(..)`; every member must be a labelled type defined in this module, or it is a compile error. `(..)` on a labelled type exports only that type and its constructor, also when its data is a union.
- Every exported name must be defined in this module.
- If the type of an exported definition, or the expansion of an exported name, uses a type defined in this module, that type must be exported too. If not, it is a compile error. A type that a macro brings in from another module must be exported by that module.

A type bound by an alias pattern is a name with no constructors of its own; exporting it with `(..)` has the same effect as exporting it alone. An exported name or annotation that depends on a value is checked after its type is complete and must not expose a private labelled type. Its source value need not be exported. Reading from another module never fixes that source's weak type; a new permanent read requires a wholly closed completed source, or the consumer uses a closed name exported by the source module (§4.7).


### 2.4.1 Unused top-level bindings

A top-level value name bound by a binding (§2.5) gives warning W0504 when it is not used. Names starting with `_` do not give this warning (§1.3). The warning does not change initialization or make `i check` fail.

A name is used when it is referenced in its module, exported from a module outside `test/`, an entry of its package (§10.2), a test, or named in `[wasm].exports`. References inside any binding, including unused bindings, function bodies, patterns, and written types, count. Merely importing a name does not use it. In test modules, an export is used when referenced by a module of the same package. Each name in a binding is checked separately; using one name does not make its other names used. A name starting with `_` suppresses its own warning.


This warning does not apply to type definitions, names, service definitions, constructors, or declarations (`@external` and `@js`). References in their written types count as uses. For an unused function in a test module, whose parameter group is `()` and whose result type is `Nil`, the warning adds: "did you forget to register this function with `test`?" The hint is not given outside test modules.


### 2.5 Top-level bindings


```
Binding     = Pattern "=" Expr .
Declaration = name ":" Type .
```

Type parameters of a function are written on the function (§6.8) or on its written type (§4.7): `id = <A>(x: A) -> A { x }`, `id: <A>(A) -> A = (x) { x }`, and the declaration `inspect: <A>(A) -> String`. A lowercase name followed by `(` is not a pattern (§7.1), so an item `id(A) = …` is a compile error.


An item that is a call, `m(args)` or `a.m(args)`, and has no `=` or `:` after its `)`, is not a binding. It is a macro call (§2.2, §14.5).


An item `name : Type` with no `=` after the type is a declaration, whatever its attributes. A declaration requires `@external` or `@js`, or expansion by a macro (§2.2). A `@js` declaration has the closed opaque type and foreign link semantics of §10.1. It is initialized once during start only if retained by the target link pass; ordinary bindings retain this section's initialization rules.


A top-level binding has the same rules as a binding in a block (§6.10). Only these things are different, and they come from its place:

- Its pattern may not begin with an `UpName`, because a top-level item that begins with an `UpName` is a type definition (§2.2).

- Its names may be used anywhere in the module, before or after it. So top-level functions may use one another.
- Its value is worked out once, when the program starts, before any entry point runs (§B.1). The order is below.

- It may not have a need (§9.2), because nothing is there to provide it. So its pattern may not fail, because a pattern that may fail needs `Assert` (§4.8 rule 10).

- If working out a value stops the program, no entry point runs.

**Order.** Working out a direct value (§6.10) runs no code, so it only reads the names outside the function bodies in it. Working out any other value may call every function it can get to, so it may read every name in it, also inside function bodies. From this:


- The **full dependencies** of a binding are all the top-level names of this module in its value, also inside function bodies, and the full dependencies of their bindings.


- The **dependencies** of a binding with a direct value are the top-level names of this module outside the function bodies in its value, and the dependencies of their bindings. The dependencies of any other binding are its full dependencies.



A binding may not be among its own dependencies. If it is, it is a compile error: `x = [x]` and `x = f()`, where the body of `f` uses `x`, are compile errors, but `f = (n) { f(n - 1) }` is not. The values are worked out one at a time, with a module after the modules it imports. In a module, the next value worked out is always the first binding, in the order of the source, whose dependencies are all worked out, as in Go. So in `a = (1, z)  y = p("y")  z = p("z")`, the order is `y`, `z`, `a`.



**Type dependencies.** A type may name a value (§3). This does not work out that value or add it to dependencies or full dependencies. It only orders type inference: a binding whose annotations use the type of a value of its module, directly or through aliases, is typed after that value's group (§4.7). A type dependency within one group, including on itself, or a cycle between groups is a compile error. The message gives the chain of dependencies.


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
they need not be exported. Each has the parameter group `()`. All declarations are checked even when unselected.
Names are unique as TOML keys; several names may select the same function.
`main` in `src/main.i` is the default entry, if present; `[entries]` does
not replace it. A package without that function has no default entry.
Entry declarations and main functions in dependencies have no effect.



`i run` and `i build` select the default entry when no entry is named.
`i run NAME` and `i build NAME` select a key of `[entries]`; an unknown key is an error listing the declared names. Program arguments to `i run` follow `--`.
Entry selection changes only the program entry, not module visibility or
host exports. The selected function uses the entry checking,
platform services and calling rules of §10.2. An unused-binding warning must
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


`wasm.synchronous` is a list of host exports and must be a subset of `wasm.exports`. A listed export that is async (§10.2) is a compile error showing the service of its need that makes it async. Otherwise the caller uses the async entry protocol when the metadata says async, even if the first call returns status 0. No source type promises synchronous host calling, and no implicit blocking adapter changes that contract. WASI retains §B.7's blocking ABI.



The targets are `js`, `nodejs`, `native`, `wasi` and `host`. `--target ilang` is an alias of `--target host`. Host artifacts keep Appendix B's format. `i run` and `i test` check against the native platform and run in the compiler's process.



| Command | Target | Checking and execution |
|---|---|---|
| `i run` | native | Check the needs of the selected entry against the platform; run in process |
| `i test` | native | Check the needs of the tests against the platform; run each test in a fresh instance |
| `i build` | the selected target; default native | Check the needs of its entries against that platform; write its artifacts (below) |
| `i check` | none | Parse, resolve and infer all source, including test bodies; check no need against a platform |
| `i check --target t` | t, with ilang read as host | Check source and the needs of that target's main, configured exports and eligible tests; write and run nothing |



WASI has no compiled test entries under §B.7: its check still checks test source,
but builds no test entry. Targeted checking checks the needs of entries against
the platform, synchronous contracts and §10.1.2's foreign
availability. Untargeted checking promises none of these: success does not
promise that any target can build or run the program. Neither check form
promises actual host grants, engine features, successful initializers, or
runtime results. Malformed manifest syntax is still a package-load error.
The target-independent source checker keeps the entry shape and interface checks, but does not reject an entry merely for needing a service. A package with only host exports uses host (or its ilang alias).


`i build [NAME] [--target t]` writes the artifacts of the selected target for the selected entry:

- `native`, the default: one executable, `build/<name>` (below). Native builds require `main` or a named entry.
- `wasi`: the WASI preview 1 command module `build/<name>.wasm` (§B.7).
- `js` and `nodejs`: `build/<name>.wasm` with its loader `name.mjs` and `name.d.ts` (§B.6.1).
- `host`, also written `ilang`: the module `build/<name>.wasm` of Appendix B, for an embedding host.

A plain `i build` writes no `.wasm` module.

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
Type       = Union .
Union      = Inter { "|" Inter } .
Inter      = Diff { "&" Diff } .
Diff       = Prim { "-" Prim } .
Prim       = FnType | DataType | NamedType | TypeCall | ValueType | Literal | "_" | "{" Type "}" .
FnType     = [ TypeParams ] FnParam [ "<-" Needs ] "->" Union .
TypeParams = "<" UpName { "," UpName } [ "," ] ">" .
FnParam    = DataType | "{" Type "}" .
Needs      = UpName | "(" [ NItem { "," NItem } [ "," ] ] ")" .
NItem      = NamedType | ".." UpName .
DataType   = "(" [ TItem { "," TItem } [ "," ] ] ")" .
TItem      = [ name "=" ] Type .
NamedType  = [ name "." ] UpName [ "(" Type { "," Type } [ "," ] ")" ] .
ValueType  = [ name "." ] name .
TypeCall   = [ name "." ] name "(" [ TypeArg { "," TypeArg } [ "," ] ] ")" .
TypeArg    = Type .
Literal    = int_lit | "-" IntPart | string_lit .
IntPart    = int_lit | "{" IntPart "}" .
```

- An `UpName` with no module name before it is a type parameter when one with that name is in scope where the type is written (§4.7, §5), and a type when not. `_` is a hole that type inference fills in.
- A `Literal` is a literal type (§4.2): `"a"`, `0`, `-1`. A `-` followed by an integer literal, also inside groups, where a `Prim` begins, is part of the literal: `-{1}` is `-1`. Every other `-` is a difference. So `Int - -1` is `Int` without `-1`, `Int - 1` is `Int` without `1`, and `-1 | 1` is a union of two literal types. A float literal is not a type, because Float has no literal types.
- A type parameter takes no arguments: `A(Int)`, where `A` is a type parameter, is a compile error.
- A lowercase `x` in a type, without `(` after it, means the type of the value named `x` at that place. The name follows value scope (§5). `m.x` means an exported value of module `m`, never a key of a value. If `m` names a value, it is a compile error. `{ x }` groups the same type. A lowercase name or member followed by `(` is a macro call, not a value type or a type application.
- A type may not name a value bound by that same pattern; a name of a parameter group in the types of that function's own head, including its `<-` and `->` parts; or its own binding's name inside that binding's value. A distinct inner binder that has entered scope may cover that name. This is a compile error even if an outer value has the same name. Those names may be named in types in the body. For example, `(x, y: x)` and `(x) -> x { x }` are errors, but `(x: Int) { y: x = x  y }` is allowed.
- A value type is not its known type (§4.8). It follows the scheme, instantiation and completion rules of §4.7. It never makes a monomorphic value polymorphic.
- A value type, directly or through names, may not occur in the data of a labelled type, an external or host declaration, or a need. Those types must be complete before inference.
- A lowercase `name`, or `a.m` with a lowercase `m`, whatever `a` is, followed by `(` is a `TypeCall`. It is a macro call in the position of a type (§14.5), and a compile error when its name is not a macro. It is in the grammar, so a type with it is read in full.
- `A | B` is a union, `A & B` is an intersection, and `A - B` is a difference (§4.4). `-` is stronger than `&`, and `&` is stronger than `|`.
- `( … )` is a **data type** (§4.1). Its items are `T`, the next position, and `name = T`, a label. Positions come before labels, and a label may not occur twice. `()` is the data type of the empty data. A `:` in a type is a compile error: labels are written `name = T`, and a function type has no parameter names.
- `( … ) -> R` is a function type whose parameter is that data type. `{ T } -> R` is a function type whose parameter is `T`: every member of `T`, after expanding names, must be a data type, and two members may not have one shape. This is the type of a function with several shapes (§4.6).
- A function type goes as far to the right as it can: `(A) -> B | C` gives `B | C`. `{ }` puts a type in a group: `{ (A) -> B } | C`, `(A) -> { (B) -> C } | D`.
- `<- N` is the **need** of a function type (§4.6, §9.2). `<- (T1, …, Tn)` lists named service types, in any order; at most one item is `..N`, a row variable. `<- N` with a bare `UpName` is the same as `<- (..N)`, and `N` must be a type parameter in scope. A missing `<-` part means "needs nothing", in every place where a type is written; `<- ()` is not written. Two items with one type constructor are merged as §9.2 says.
- `<A, N>` before the parameter part binds type parameters for the rest of that function type (§4.7). `<>` is one token, so an empty list of type parameters cannot be written.
- `Opaque` is not a type. It is only the right side of a type definition (§4.3).
- A `{ }` in a type groups exactly one type. Breaks in data types separate items; breaks directly in type groups are compile errors.

## 4. Types

### 4.1 Types are sets of values

Every type is a set of values.

- Values of different type constructors are never the same.
- Values of different forms of one type constructor, such as `List(Int)` and `List(String)`, may not be told apart at run time (§7.1).
- **Data** is a finite map from keys to parts. A key is a position, counting from 0, or a label, a `name`. The **shape** of data is its number of positions and its set of labels. Data of one shape is one type constructor; data of different shapes are different type constructors. `()` is the empty data, of the empty shape.
- Each labelled type (§4.3) is a type constructor of its own. Its values are never data, and data is never a value of a labelled type.
- The values of a function type are functions. All function types are one type constructor.

A **member** of a type (§4.4) is a type constructor, its type arguments, and a **value set**: the values of that type constructor that are in the member. Only `Int` and `String` may have a value set that is not all of their values (§4.2). For every other type constructor, the value set is all of its values.

### 4.2 Built-in types

These types may be used in every module and named through `i/core`, for example `core.Int` after `(core = i/core)`. So a module that defines a type with the name of a built-in type can still name the built-in type. The prelude types may be named through `i/prelude`.

| Type | Values |
|---|---|
| `Int` | Whole numbers of any size |
| `Float` | Finite IEEE 754 binary64 numbers |
| `String` | Lists of Unicode scalar values, kept as UTF-8 |
| `List(A)` | Lists that may not be changed |
| `BitArray` | Lists of bits; the number of bits may be any number |
| `UtfCodepoint` | One Unicode scalar value |
| `Never` | No values |

These prelude types are normal type definitions (§4.3):

```i
True = ()
False = ()
Bool = True | False
Nil = ()
Ok(A) = (A)
Error(E) = (E)
Result(A, E) = Ok(A) | Error(E)
```

The types `Fail` (§9.3), `Assert` and `AssertionFailure` (§9.6) are prelude types too. The types `Boundary` and `Continuation` are in §9.4. The types `Key` and `Keyed` are in §9.8. `Request(A)` and `Ticket` are built-in types, with operations in `i/request` (§9.11). `Request(A)` contains request handles whose answer belongs to `A`. `Ticket` is an answer-erased handle. Neither has public constructors.

#### Literal types

Every value of `Int` and of `String` is also a type, the **literal type** with only that value. It is written as the literal: `1`, `-1`, `"a"`. `1 <: Int`, and `"a" <: String`. Two Int literal types are the same when their numbers are equal, so `0x10` is `16` and `-0` is `0`. Two String literal types are the same when their scalar values are equal (§11.4).

The value set (§4.1) of an `Int` or `String` member is one of these:

- all the values: `Int`, `String`;
- a finite set of values: `"a" | "b"`;
- all the values but a finite set: `String - "a" - "b"`.

Union, intersection and difference of these sets give a set of the same kind, so the rules of §4.4 and §4.5 always have an answer, and inference still finds principal types (§4.7). `Float`, `BitArray` and `UtfCodepoint` have no literal types.

Values are never changed, so a literal type is never made wider: `x = "a"` gives `x` the type `"a"`, and `[200, 404]` has the type `List(200 | 404)`. Such a value may be used where a `String` or an `Int` is needed, because `"a" <: String`. Operators give the whole type: `1 + 1` has the type `Int` (§6.9).

### 4.3 Type definitions

```
TypeDef     = UpName [ TypeHead ] "=" TypeRhs .
TypeHead    = "(" TypePattern { "," TypePattern } [ "," ] ")" .
TypeRhs     = "Opaque" | Type .
TypePattern = UpName | "_"
            | [ name "." ] UpName "(" TypePattern { "," TypePattern } [ "," ] ")"
            | name "." UpName
            | "(" [ TPItem { "," TPItem } [ "," ] ] ")" .
TPItem      = [ name "=" ] TypePattern .
```

A type definition has one form, `Name [ (params) ] = right side`. Its kind is decided by the outermost written form of the right side, before any expansion, in this order:

1. **Alias pattern.** The right side, after removing groups, is a value type (§3).
2. **Opaque type.** The right side is the word `Opaque`, not in a group.
3. **Labelled type with data.** The whole right side is one data type, not in a group.
4. **Name.** Any other right side.

- For kinds 2 to 4 the head is a list of **type parameters**: distinct, unqualified `UpName`s with no arguments, which do not already name an accessible type.
- `T = Opaque` defines a new **opaque type**, a labelled type whose values i code cannot make or take apart. They come from the host (§10), including `@js` declarations (§10.1.1). `Opaque` anywhere else, and a type named `Opaque`, are compile errors.
- `T = D`, where `D` is a data type, defines a new **labelled type** with the data `D`, and its one **constructor**, also named `T`. The constructor is the function `(D) -> T(P1, …)`, or, when `D` is `()`, the value `T`. Two labelled types are different types even when their data is the same. The module defines the constructor name `T` in its space of value names (§2.3).
- Any other right side defines a **name**: a transparent alias for the type on its right. A name may not occur in its own expansion, directly or through other names or alias patterns. The data of a labelled type is not part of the expansion of a name, so a name may reach itself through the data of a labelled type: `Tree(A) = Leaf | Node(A)` with `Node(A) = (Tree(A), A, Tree(A))` and `Leaf = ()` is correct, but `T = List(T)` is not.
- In a name, every parameter must be used, and every type parameter on the right must be a parameter of the head. In a labelled type, a parameter need not be used. A parameter of a labelled type that its data does not use, and every parameter of an opaque type, does not change (§4.5 rule 6); the other parameters change as the data says.
- A **sum** is a name for a union, or a labelled type whose data is one item of a union type. Its members may be any types, labelled or not, also built-in types and data types. Values of a sum are told apart by type tests and constructor patterns (§7.1), so every two members of a sum must have different type constructors (§4.4). Members that would share a type constructor are each wrapped in a labelled type. One labelled type may be a member of several sums.
- `{ D }`, a data type in a group, is a name for that data type: `Point = { (Int, Int) }` makes `Point` mean `(Int, Int)`, while `Pair = (Int, Int)` makes a new labelled type.
- An item `T(P1, …, Pn) = v` is an **alias pattern** exactly when its written right side is a value type (§3), after removing groups. This classification is made before expanding the right side; it never depends on imports or on what a macro returns there.
- In an alias pattern, the unqualified head names an existing type, with the written arity; the item does not define that head. It may be a labelled or built-in type, or a name expanding to one named type or data type. In a head name each parameter is used at most once, only as a named argument or a data part. Constant subtrees of that name require an equal, fully determined source part.
- A bare unqualified UpName in a pattern always defines a new zero-parameter name. Its part of the source type must be fully determined (§4.7). It may not duplicate a definition or conflict with an imported or built-in type. It never tests an existing type with that name.
- `_` in an alias pattern skips a part and binds nothing; it is not an inference hole. For a new read after source completion, the whole source type must first be closed (§4.7); a skip cannot hide an open argument from that check. Elsewhere in a type, `_` remains a hole. Shared type variables remain shared even if one occurrence is skipped.
- A named pattern with arguments, or a qualified named pattern, requires exactly one member with that type constructor and matches its arguments. A data pattern requires one member that is data of the same shape and matches its parts at their keys. No union member is discarded to make a match. A mismatch is a compile error. Function, literal, union, intersection, difference, value-type and macro-call patterns are compile errors.
- Every pattern and every value-dependent name is checked, also when unused. Its demands are processed at the completion of their source (§4.7), and its results are assembled after its name dependencies are complete. A new read after source completion follows the closed-source rule in §4.7. A definition read may not choose an instance of a generalized variable to make its pattern fit.
- Blocks have no type definitions. An item beginning with `(` or `{` is a value binding; `m.T(…) = v` is also a value binding under §2.2. Import `T` to use it as an alias-pattern head.

```i
Some(A) = (A)
None = ()
Option(A) = Some(A) | None
User = (name = String, age = Int)
Pair(A, B) = (A, B)
Point = { (Int, Int) }
Meters = Int
Element = Opaque

Codec(D, E) = (decode = (String) -> D, encode = (E) -> String)
size = Codec(decode = (s) { string.length(s) }, encode = (n: Int) { int.to_string(n) })
Codec(Size, _) = size
```

Here `Pair(Int, Int)` is a labelled type, not `(Int, Int)`; `Point` is `(Int, Int)`; and the alias pattern makes `Size` the type `Int`, leaving the `encode` part unread.

### 4.4 Union, intersection, and difference

- The values of `A | B` are the values of `A` and the values of `B`. Order has no effect, `A | A` is `A`, and `A | Never` is `A`.
- Members of a union with the same type constructor are put together into one. For `Int` and `String`, the value set of the member is the union of their value sets: `"a" | "b"` is one member, and `"a" | String` is `String`. `T(A1, …) | T(B1, …)` is `T(C1, …)`, where for each parameter of `T` (§4.5 rules 6 and 7), `Ci` is `Ai | Bi` when the parameter goes in the same direction as the type, and `Ai & Bi` when it goes in the other direction. When the parameter does not change, `Ai` and `Bi` must be equal, and `Ci` is that type; if they cannot be equal, it is a compile error. Data of one shape is one type constructor, whose part under each key goes in the same direction as the data. All function types are one type constructor: `(P) <- N -> R | (Q) <- M -> S` is `(P & Q) <- N ∧ M -> R | S`, where `N ∧ M` is the merge of the two needs (§9.2). When `P` and `Q` have no shape in common, `P & Q` is `Never`, and the function cannot be called. So `List(Int) | List(String)` is `List(Int | String)`.
- The **members** of a type are the types that make it up as a union, after these rules are used. So no two members of a type have the same type constructor. A type variable may be a member.
- The values of `A & B` are the values that are in both `A` and `B`. `{ A | B } & C` is `A & C | B & C`. When `T` and `U` are not type variables, `T & U` is `Never` when they have different type constructors; when they have the same one, it is that type constructor with each pair of arguments put together the other way round: `&` where the parameter goes in the same direction, `|` where it goes in the other direction, and equal where it does not change. For `Int` and `String`, the value set is the values in both; when there are none, `T & U` is `Never`. So `"a" & String` is `"a"`, and `"a" & "b"` is `Never`. `a & T`, with a type variable `a`, stays as it is.
- Intersections come from inference. `(x) { x + 1  x }` has the type `(a & Int) -> a`: the value that comes in is given back, and it must be an `Int`.
- `A - B` takes out of `A` every member whose type constructor is the type constructor of a member of `B`. For `Int` and `String`, it takes out only the values of the value set of that member of `B` (below). These rules work it out:
  - `{ A | B } - C` is `A - C | B - C`, and `A - { B | C }` is `A - B - C`.
  - When `T` and `U` are not type variables: `T - U` is `T` when they have different type constructors. When they are both `Int` or both `String`, `T - U` has the values of the value set of `T` that are not in the value set of `U`, and is `Never` when none are left: `String - "a"` is every string but `"a"`, `{"a" | "b"} - "a"` is `"b"`, and `"a" - String` is `Never`. For every other type constructor that is the same in both, `T - U` is `Never`, and `T <: U` must be true (below).
  - When `T - U` is `Never`, `T <: U` must be true. This is a constraint for inference, like any other. When it cannot be true, it is a compile error: for example `List(Int | String) - List(String)`.
  - The right side of `-` may not have a type variable as a member. `a - U`, with a type variable `a`, stays as it is until `a` is known.
- So `{ Int | String | Nil } - String` is `Int | Nil`, `A - A` is `Never`, and `{ Yield(a) | n } - Yield(a)` is `n - Yield(a)`.
- `{ Int | String | Nil } - "a" - 1` is `Int - 1 | String - "a" | Nil`.
- Unions, intersections, and differences may be used in every place where a type may be used. In a written type, a union with two or more type variables as members may only be in an output place, and an intersection with a type variable only in an input place (§4.5 rule 6), because inference cannot choose between them in the other places. The parameter of a function is an input place; its `->` part, the type at the top of a binding pattern, and the data of a type are output places. The parts of data keep the direction of the place of the data. The restrictions on variable members apply to expanded annotations and shown types, including after substitution and completion.

### 4.5 Subtypes

`A <: B` says that every value of `A` is a value of `B`. These are the rules:

1. `Never <: A`, and `A <: A`.
2. `A <: A | B`. `A | B <: C` when `A <: C` and `B <: C`. `A & B <: A`. `C <: A & B` when `C <: A` and `C <: B`.
3. `A - B <: A`. `X <: A - B` when `X <: A` and `X` and `B` are **disjoint**: every member of `X` and every member of `B` have different type constructors, or are both `Int` or both `String` with value sets that have no value in common. So `"a"` and `"b"` are disjoint, and `"a"` and `String - "b"` are not. Only the members themselves are compared, not their type arguments: `(1, 2)` and `(3, 4)` are not disjoint. A type variable as a member is never disjoint from anything.
4. `{P} <- N -> R <: {Q} <- M -> S` when `Q <: P`, `R <: S`, and `N` **fits in** `M`: for every type constructor `C` with an entry in `N`, `M` has an entry for `C` and `M_C <: N_C`, or `N` and `M` have the same row variable and it carries `C`. Here `{P}` is the parameter type, a data type or a union of data types, and needs are compared in the reverse direction of values.
5. Data of the same shape: `D1 <: D2` when the part under every key of `D1` is a subtype of the part under the same key of `D2`. Data of different shapes are disjoint.
6. A type parameter of a labelled type changes in the same direction as the type, in the other direction, or not at all. This is decided from the definition: a parameter that is only in output places goes in the same direction; one that is only in input places goes in the other direction; one that is in both does not change. The data of a labelled type is an output place, the parameter of a function type is an input place, and a place inside an input place is the other way round. A parameter that the data does not use, and a parameter of an opaque type, does not change.
7. `List(a)` goes in the same direction as `a`. In `Continuation(A, R, N)`, `A` goes in the other direction and `R` and `N` in the same direction. `Boundary(R, N)` does not change.
8. A member of `Int` or `String` is a subtype of a member with the same type constructor when every value of its value set is in the value set of the other. So `"a" <: "a" | "b" <: String`, and `String - "a" <: String`, but `String` is not a subtype of `"a" | "b"`, and `String - "a"` is not a subtype of `"b"`.

Two types are **equal** when each is a subtype of the other. A value of a subtype may be used in every place where a value of the type is needed.

`Key(A)` (§9.8) does not change. `Request(A)` (§9.11) goes in the same direction as `A`. The variance of task types follows their definitions (§9.9).

### 4.6 Function types

The function type `(D) <- N -> A` has one parameter, a data type `D`; a **need** `N` after `<-`, the services that it takes from where it is called (§9.2); and a **value** `A` after `->`, the type of the value that it gives back. Errors are the need entry `Fail(E)` (§9.3), so `Effect<A, E, R>` of Effect-TS is `() <- (..R, Fail(E)) -> A`. A missing `<-` part means "needs nothing".

The shape of `D` is part of the type: its positions and its labels. There is no currying and there are no default parameters. `{ D1 | … | Dn } <- N -> A`, where the `Di` have different shapes, is the type of a function with several shapes (§6.12): calling it with data of the shape of `Di` needs that data to be in `Di`, and gives `A`. Such a type is a subtype of each of its shapes, `{ (Int) | (Int, Int) } -> R <: (Int) -> R`, so a function with several shapes may be given where a function of one of them is needed.

`<A, N>` before the parameter part makes the type polymorphic (§4.7).

### 4.7 Type inference

Types are worked out by inference with subtypes (algebraic subtyping).

- Type annotations may be left out everywhere for inference, but not on `@external` or `@js` declarations. Some uses of a name need a known type, which may need an annotation (§4.8).
- Foreign linking (§10.1.2) does not change inferred source types or remove inferred needs.
- For every binding, the compiler must find the principal type: the most general type, of which every other possible type is an instance. MLsub (Dolan and Mycroft, POPL 2017) and MLstruct (Parreaux and Chau, OOPSLA 2022) give ways to do this.
- A type is made simpler before it is shown: equal parts are put together, and type variables that have no effect on the meaning are taken out. This never changes which programs are correct or what they do. A type scheme that is used as a known type (§4.8) is in this simpler form. A type variable that is not generalized is shown as the union of the types that come to it. A union that is the expansion of a declared name is shown by that name.
- Annotations are used in inference, and the inferred type must agree with them. A type variable in an annotation may become a more exact type.

**Type parameters.**

- A function expression may begin with `<A, N, …>` before its parameter group (§6.8), and a function type may begin with it before its parameter part (§3). The names are distinct unqualified `UpName`s that do not name a type that may be written with no module name in the module (a type of the module, an imported `UpName`, or a built-in type). A type definition declares type parameters in its head (§4.3). The parts of an alias pattern are not type parameters: its bare UpNames define names.
- On a function expression, the type parameters may be used in its parameter group, its service pattern, its `->` part and its body, also in nested functions and annotations there. On a function type, they may be used in the rest of that type.
- A type parameter used where a need is written (`<- N`, `<- (..N, …)`) is a **row variable**: it stands for a whole set of services. Any other use makes it an ordinary type variable. Using one name both ways is a compile error.
- On a function expression, a type parameter is a type variable of that function, and it is generalized with it. Inference may make it more exact: `id = <A>(x: A) -> A { 1 }` is correct.
- A written function type with type parameters in a parameter, in the data of a type, in a type definition, or in an annotation is a **polymorphic type**, as `Show = (write = <A>(A) -> Nil)`. A value checked against it is checked with a new rigid type for each of its parameters, which may not escape; each use of a value of that type instantiates it again. Inference never guesses a polymorphic type: a parameter gets one only from a written type.
- An `UpName` in a type that is neither a type nor a type parameter in scope is a compile error. The message says how to declare it.
- A bound on a type variable is written as an intersection where it is used, as `x: A & Show`. Capabilities are passed as parameters or as services; none is passed implicitly.

**Generalization.**

- **A binding is generalized when its value is a direct value** (§6.10): the type variables of its type that are not in the types of names from outside it become general, so each use of the name may give them different types. A binding whose value is not a direct value is not generalized, because working out its value runs code, and that code may make a value for one type only (for example with `control.suspend`). This is the value restriction of ML, which is known to be safe. Functions are direct values, so every function binding is generalized.
- A type variable that is not generalized is one type for all uses of the binding.
- Parameter groups (§6.11) are matched like bindings, but they are not generalized. Their values are not known where they are written, so each of their names has one type in the whole branch. So `(f) { (f(1), f("a")) }` needs `f` to take both `Int` and `String`.
- Inside its own value (§6.10), a name of a binding is not general: all its uses there have one type. It becomes general when the binding is generalized.
- A **group** is a set of top-level bindings that use one another, directly or through other bindings of the group. The bindings of a group are worked out together and then generalized together, after the groups that they use. Inside the group, their names are not general.
- A type that has itself inside it, through a type variable, is a compile error. The parts of data count as places inside a type constructor.
- Simplification and type display preserve type variables that affect invariant arguments, even when their input and output forms print alike. Reading a type never generalizes a weak variable.
- A long union of literal types may be shortened in messages and hovers, as `"a" | "b" | … (40 more)`, but never in a type scheme or in the interface of a package. An inferred type may have literal types in it, also in the interface: an exported constant or an unannotated exported function whose value is a literal has that literal type, so giving one more literal as a value changes the interface. Exported functions should have written types, as the standard library has.

**Reading types of values.**

- In an annotation, a completed generalized value is read as a fresh instance of its scheme (§4.8 rule 3), separately for each occurrence. A parameter name or local monomorphic value shares its one type. Generalized local schemes also keep their non-general variables shared. A completed top-level value uses its fixed non-general parts; reading an unfixed part is a compile error. Reading through a local does not copy such a part or allow a name to escape from it. A value type never generalizes its source or its annotated binder.
- In a name or alias pattern, every used value-derived part must be closed: it has no generalized variable, unresolved type variable, unknown top type or hole. The name's own declared parameters remain parameters. A skip demands no contents, but the shapes needed to reach the used parts must also be determined.
- A top-level value's group has one **completion point**, after its values are typed and linked and its generalization is decided. All value-type demands written in its own module are collected before inference, including annotation reads and unused type definitions. An annotation read takes part in fixation like a name read; its quantified parts still use fresh scheme instances. Ordinary group checks finish first. All demands at this completion point read one unchanged snapshot and are checked in one batch before any permanent name is published. At this point their weak parts are fixed as follows. A demand in another module never fixes them.
- For a demanded non-general variable, collect all types that come to it, including through variable relationships. Put them together by union, replacing the non-general variables needed inside them by their fixed types. Pure cycles of variable relationships count once. A structural recursive type, an unfilled part, a generalized variable needed inside this replacement, or a part that cannot be made closed is a compile error. An explicit closed `Never` is allowed; the absence of any information does not invent `Never`. Literal types are never widened.
- Fixing adds both subtype constraints between the variable and the resulting closed type, keeping every previous constraint. Every needed variable is fixed together, and the usual occurs and place rules apply. If these equations cannot hold, it is a compile error. Pattern shape checks use the original argument variables, so skipping a distinct argument leaves it shared and unfixed. The pattern cannot supply a type to fill an unknown.
- A group that constrains an earlier non-general variable that was not fixed cannot fix its own non-general parts. A demanded part carrying such an earlier variable also cannot be fixed. This conservative rule makes the result independent of unrelated groups' inference order.
- From then on, later uses must agree with each fixed part. A use that would widen it incompatibly is a type error with a note at the place that fixed it. Write a wider annotation at the source when needed. For example, reading `r = task.ref(1)` may fix its invariant argument to `1`; write `r: Ref(Int) = task.ref(1)` to allow other Int values.
- In another module, or when a new definition read is performed by a later group, an alias pattern or other permanent value-derived definition must start from a wholly closed completed source type. A fixed component behind an open enclosing type is not sufficient, even when the open arguments are skipped. This read adds no constraints to the source. If the source remains open, the error says to use the closed name made by its source module and to export it when needed. Annotation instantiation and current-group local monotype sharing keep their rules above. Adding foreign fixation demands cannot change the source module's completion result. Ordinary expression constraints and local monomorphic sharing remain shared as elsewhere in this section; this rule does not fix the display of every uncompleted weak part.
- A permanent name denotes the completed closed part. It does not change when later bindings are checked. A same-module top-level pattern is handled at its source's completion, regardless of where the item is written. Names with several sources assemble already closed per-source results, not saved partial source types. Definitions are completed before their dependents. Importing or using the closed name later does not read its source value again.
- The whole annotation after value types and names are read must obey §4.4's rules of places. A value-type spelling does not hide variable members from that check.

### 4.8 Typing rules

`Γ ⊢ e : A <- N` says: with the names in `Γ`, the expression `e` has the type `A` and needs the services `N` (§9.2), errors included as `Fail(E)` (§9.3). `Γ` gives every name a type scheme `∀a1 … an. T`. A program is correct when every binding has a type by these rules. Every rule may use any `N`; the parts of one rule use the same `N`, unless the rule says otherwise. **Every value that an expression can have at run time is in its known type** (below); selection (rule 5a), narrowing (rule 12) and keys (rule 13) depend on this.

1. **Subtypes.** If `e : A <- N`, `A <: A'`, and `N` fits in `N'` (§4.5 rule 4), then `e : A' <- N'`.
2. **Literals** have the types of §6.3.
3. **Names.** If `Γ(x) = ∀a1 … an. T`, then `x : T` with each `ai` changed to any type.
4. **Functions.** Let the parameter group be the data pattern `q`, and the service pattern `x1: T1, …, xk: Tk` (§6.8). `q` is fitted to a data type `P` (below). If `Γ` with the names of `q` and `xi : Ti` gives `body : A <- M`, the function has the type `(P) <- M' -> A`, where `M'` is the merge (§9.2) of `M`, every `Ti`, and `Assert` when `q` may fail for `P` (§7.3). With `..env: N` in the service pattern, `M` must fit in `(..N, T1, …, Tk)`, and `M'` is `(..N, T1, …, Tk)`. Making the function needs nothing.
5. **Calls.** If `f : {P} <- N -> A` and the argument data `d : D` with `D <: P`, then `f(d) : A <- N`.
   **5a. Selection.** When `f` is a name with a selection table (§6.12), its type at the call is the type of the selected branches, computed from the known types of the arguments before inference constrains the call.
   **5b. Provide.** For `e <- s`, where `e : A <- N` and the provided data has keys `K` with types `T_k` (§9.2): for every `k` in `K` that `N` has, `T_k <: N_k`. The provide has the type `A <- N'`, where `N'` is `N` without the keys `K`, merged with the need of working out `s`. A provided key that `N` does not have gives a warning. When `N` has a rigid row variable, a key that it may carry and that `N` does not state is a compile error.
6. **Data and lists.** If every item `ei : Ti` at key `ki`, the data has the data type with `Ti` at `ki`; a spread contributes the parts of the type of its base at their keys. If every `ei : T` and `r : List(T)`, then `[e1, …, en] : List(T)` and `[e1, …, en, ..r] : List(T)`.
7. **Constructors** are functions from their data type to their type, or values when the data is `()` (§6.7).
8. **Operators** are not functions. Each has the types of §6.9. `a |> f(b)` has the type of `f(a, b)`.
9. **Blocks.** `{ s1 … sn e }` has the type of `e`, with the names bound by `s1 … sn` (§6.10). The need of a block is the merge of the needs of its statements and of `e`.
10. **Bindings.** For `p = e` with `e : A`: `p` is fitted to `A` (below). If `p` may fail for `A` (§7.3), then `Assert` is in `N`.
11. **Groups.** This rule is for a shape group of a chain (§6.12) with more than one branch or with a guard. Let it be called with data of type `B`: in a match `e |> C` with `e : A`, the one-position data `(A)`; in a call, the argument data type. The parameter patterns (each alternative of `|` counts) are looked at by **place**. The top place is the argument data. Below a pattern at a place, its parts are at places of their own: the part under each key of a data pattern, the part under each key of a constructor's data, and the parts and the rest of a list. Places below different constructors are different places. At each place, let `B` be the type of the values there (the argument data at the top place, and else the part type of the pattern type above it), and let `P` be the union of the pattern types (below) of the patterns at that place that are not catch-alls. Patterns at one place with the same type constructor use the same new type variables, so `Ok(v)` and `Error(e)` at one place give `Ok(b1) | Error(b2)`, and `Some(v)` and `None` give `Some(b) | None`. A type test `x: T` at a place also needs `B <: T | r - T`, with a new type variable `r`: it takes every value whose type constructor is that of a member of `T`, and that, for a member of `Int` or `String`, is in the value set of that member (§7.1), so those values must be in `T`. So `v |> (x: Box(Int)) { … } | (y: Box(String)) { … }` with `v: Box(Int | String)` is a compile error. A data or constructor pattern with `..` counts as having a catch-all at the places of the keys that it leaves out. If some branch without a guard has a catch-all at that place or at a place above it, then `B <: P | r - P`, with a new type variable `r`: a value that no pattern takes may still come there. If not, then `B <: P`. So a group with only literal branches and no catch-all takes only those values: `(0) { "zero" } | (1) { "one" }` has the type `(0 | 1) -> "one" | "zero"`. In a match `e |> C` whose subject already has a type with values that are in no literal of `P`, so that `B <: P` is not true only for that reason, as `(n: Int) { n |> (0) { "zero" } | (1) { "one" } }`, the error is the one of §7.4, with a value that no branch matches, and no other error is given for it. A chain that is not the right side of a match gets its values later, through the type of its parameter: from a call, or from the type it is given to, as in `list.map(xs, (0) { "zero" } | (1) { "one" })` with `xs: List(Int)`. Then there are two errors: a type error where the values come, and the error of §7.4 at the chain. A catch-all at a place gets the type of rule 12. Every guard has the type `Bool`, each branch body has its own type `Ri`, and the need of every guard and body is part of the need `N` of the chain. The group has the type `(B) <- N -> R1 | … | Rn`, and a match has the type `R1 | … | Rn`. Making the chain needs nothing, as for a function in rule 4. Coverage (§7.4) is checked with the types that inference finds.

    A group with one branch and no guard is not typed by this rule. It is the function of rule 4: its parameter group is fitted to the parameter type (below), it is not checked for coverage, and `Assert` is in its need when the pattern may fail. A match `s |> (p) { e }` with such a chain is typed as the binding `p = s` followed by `e` (rule 10), with the one difference that `p` is not generalized, as a parameter is not. So the subject `s` gets the expected type that a binding gives its value (below).

12. **Narrowing.** Let a catch-all be at some place in a parameter pattern of a group, and let `B` be the type of the values at that place. A type constructor `T` is **taken** at that place when every value that the branch's pattern matches, with a value of `T` at that place, is matched by an earlier branch of the group without a guard. In the same way, a value `v` of an earlier `Int` or `String` literal pattern at that place, and a member of the type of an earlier type test at that place, is **taken** when every value that the pattern matches with it at that place is matched by an earlier branch without a guard. The catch-all gets the type `B - H`, where `H` is the union of the taken members: each taken type constructor with new type variables as its arguments, the literal type of each taken value, and each taken member of a type test. So in `s |> ("a") { 0 } | (rest) { f(rest) }` with `s: String`, `rest` gets `String - "a"`. Guarded branches take nothing.
13. **Keys.** For `v.k` and `v.f`, `v` must have a known type, and every member of it must be data with that key, or a labelled type whose data has that key and whose constructor may be used here (§2.4). The inferred result is the union of those parts with the type arguments from the corresponding bound forms, or the constructor-pattern arguments already supplied by §7.5. The inferred type of `v` must fit the bound. If not, it is a compile error. The bound takes all Int and String values at covariant places, removes exclusions of only some of those values there, and takes fresh holes at noncovariant places. Use this bound also when a known function type constrains a call. Known key results are obtained separately from the original known facts, not by narrowing the inferred result to them. A known type may have fewer values than the inferred type: after `x = "a" = s` with `s: String`, `x` has the type `String` and the known type `"a"`, and `(x, 0).0` is correct.
14. **Generalization.** A binding whose value is a direct value is generalized (§4.7) after its value is typed. Other bindings and parameter groups are not.

A **catch-all** is a `name`, a discard, or a conjunction `p1 = … = pk` whose parts are all catch-alls.

To **fit** a pattern `q` to a type `B`, in a binding or in the parameter group of a group with one branch and no guard: if `q` is a catch-all, its names get `B`. If not, `B <: Q`, where `Q` is the pattern type of `q`, and the parts of `q` are fitted to the parts of `Q` given below. A constructor pattern `C(…)` with pattern type `Q` needs only `B <: Q | r - Q`, with a new type variable `r`: it may fail, and when the known type of the value is disjoint from `Q` the compiler warns that it can never match. Fitting never adds `Assert` by itself; rule 4 and rule 10 do. In any other group, rules 11 and 12 take the place of fitting.

Only a fitted group may have alternatives `q1 | … | qn` in a parameter item. If one alternative is a catch-all, the names get `B`. If not, `B <: Q1 | … | Qn`, where `Qi` is the pattern type of `qi`, and alternatives with the same type constructor use the same new type variables. The parts of each `qi` are fitted to the parts of its `Qi`, and a name gets the union of its types in the alternatives (§7.2). In alternatives, `x: T` is a test, not a type annotation (§7.1), so `T` may not have a type variable as a member. The alternatives may fail for `B` when together they do not match every value of `B` (§7.3); then `Assert` is in the need of the function (rule 4).

The **pattern type** of a pattern that is not a catch-all, and its parts, are:

- A literal of `Int` or `String`, also `-` and an integer literal: in a group typed by rule 11, its literal type (§4.2); where it is fitted, `Int` or `String`, so that every value of that type may come there and the pattern may fail (§7.3). At a place whose type is shared with another place, or inside one, also `Int` or `String`: an item of a list pattern, because all the items of a list have one type, and a key of a constructor whose written data has a type parameter that occurs more than once in it (counting through names), as both parts of `Twin(A) = (A, A)`, and the parts of the data of `W(A) = ((A, A))`. A literal type at one such place would be a type for all of them. So when that type is not written and does not come from the subject, a group that matches those places only with literals is not covered (§7.4): `(xs) { xs |> ([]) { 0 } | (["a", ..]) { 1 } | (["b", ..]) { 2 } }` gives `xs` the type `List(String)`, and the error names a value such as `[""]`. With `(xs: List("a" | "b"))` written, the same chain is covered. A float literal: `Float`.
- `C(…)`: `C(b1, …, bm)`, the labelled type `C` with a new type variable for each type parameter. The part for each item is the part of its data type at that item's key, with these variables.
- A data pattern: the data type of its shape, with a new type variable at each key. The part for each item is the variable at its key; a rest `..r` gets the data type of the remaining keys.
- `[p1, …, pn]` and `[p1, …, pn, ..r]`: `List(b)`, with a new type variable. The part for each `pi` is `b`, and `r` gets `List(b)`.
- `"lit" <> r`, `r <> "lit"`, and `"a" <> r <> "b"`: `String`, and `r` gets `String`.
- A bit array pattern: `BitArray`. The parts get the types of their formats (§8.2). A literal value in a segment also has the type of its format, not its literal type.
- A conjunction `p1 = … = pk`: the intersection of the pattern types of its parts that are not catch-alls, and each name part gets that type.
- `x: T`: `T`, with new variables for holes and value types read as §4.7 says; `x` gets that type.

A value type in an annotation uses the inferred type of its source, not a narrower known type at the use. A completed scheme gives its known structural form, with `_` for general variables. For a local source still being inferred, use its stored structural monotype with unresolved variables erased to `_`. A written type on the source name is part of that monotype. The known type of a matched value does not supply a type for a name inside its pattern. Do not infer a smaller shape from temporary lower bounds. Later literal tests or branch narrowing do not refine this shape. Thus `((a = Int, b = String) = d) { q: a = d.a  q == 1 }` gives `q` the known type `Int`. For `x = "a" = s` with `s: String`, `x`'s inferred type is `String` even though its known type is `"a"`; an annotation naming `x` uses `String`.

The parts of data are covariant in the hidden-input and whole-literal operations below; invariant arguments inside them still count in both directions. An item written in a data literal gets the matching expected part, when guaranteed. The base of a spread gets no expected type.

The **known type** `K(e)` of an expression is found from the parts of the expression, left to right, and from the expected type of its place (below). Where no case below gives it, `e` has no known type.

- A data literal has the data of the known types of its items, with `_` for an item that has none; a spread contributes the known parts of its base.
- A literal, also `-` and an integer literal, also inside groups, has its type (§6.3), and a bit array has `BitArray`. A list has `List` of the union of the known types of its parts when they all have one, and `List(_)` when not. So `[]` has `List(Never)`.
- An operator expression has the result type of §6.9; `a |> f(…)` is the call `f(a, …)`.
- A match `e |> C` has the union of the known types of its branch bodies when they all have one.
- The names of a parameter group get known types as names in patterns (below), but the argument data has no known type. So `x: T` gives `x` the type `T`, and a parameter with no type has no known type. A name of a service pattern `x: T` has `T`.
- A built-in name, an imported name, and a member `m.x` of an imported module have their type scheme. So does the name of a generalized binding (§4.7), local or top-level, where it is used outside its own value and outside its group. A type variable in such a scheme that is not generalized is replaced by the matching part of the known type of the binding's value, or by `_`. A constructor has its function type, or its type when it is a value.
- A function expression has the type made from: for its parameter data, the shape of its parameter group, with the type written at the top of each item (as in `x: T`), or else the type it gets from an expected type (below), or else `_`; and for the result, the written `->` part, or else the known type of its body, or else `_`. A chain literal has, for each shape group, a known function type made in the same way, with the union of the known types of its branch bodies as the result when they all have one; a chain with several groups has the known multi-shape type `{ D1 | … | Dn } <- N -> R` of §6.12. When the parameter of a group has a known type `B`, from a written type or from an expected type, every parameter pattern of the group is a pattern fitted to a value with the known type `B`, as in a match (below), and the narrowing of rule 12 is used in the same way. So with `opts: List(Option(User))`, `list.map(opts, (Some(u)) { u.name } | (None) { "" })` is correct.
- A name used inside the value of its own binding, and a top-level name of the same group, get their known types only from what is written, because their values are not yet known there: from `x: T` at the top of the binding's pattern when it is there, or else from what is written in the value, with `_` for the rest. So a function that uses its own result in `.f` or `.k` needs a written `->` part. Every other name bound by a pattern gets its known type as below.
- A known type may have `_` in it, for a part that is not known. A part that is `_` has no known type, but the rest of the type is still known. So the shape of a function is known even when its types are not.
- A call `f(d)`, also when written with `|>`, has a known type when `f` has a known function type, and every type variable of its result gets a type when its parameter type is matched with the known type of the argument data (below). Then it has the result type with those types put in. A constructor with data is a call.
- To **match** a type `P` with a known type `K`: when `P` is a type variable, `K` is one of its types. When `P` is a named type, a data type, or a function type, and `K` has one member with the same type constructor, the parts are matched: the type arguments, the parts at each key, or the parameter and result types. When `P` is a union, each member of `P` that is not a type variable is matched with the member of `K` that has its type constructor, if there is one, and each type variable in `P` is matched with `K` less the other members of `P` (§4.4). A type variable in `P` that is matched with a `_` in `K`, or with a type variable that is generalized in `K`, gets `_`, because that part may be any value. When `P` is an intersection, each of its members is matched with `K`. Before matching, a difference in `K` is replaced by its members (§4.4), and an intersection `a & T` by `T`. When a type variable gets more than one type, it gets their union. So matching `(a | Nil)` with `(User | Nil)` gives `a` the type `User`.
- **Expected types.** An expression may have an **expected type** from its place, as in TypeScript:
  - An item of the argument data of a call (also with `|>`) whose callee has a known function type has the type of the parameter data at that key. Its type variables are put in when matching the parameter type with the known types of the items before it fixes them. Function expressions that have a parameter with no written type, and names with a selection table of more than one branch, are left out of this matching, and come last: their expected types use all the other items. A function given as an argument may give values to the called function that the other arguments do not show: as its result, as the argument of a function it is given (a parameter of a parameter), or to a service of its need. A service in the called function's own need gives it values that come from the caller's provide. And a function that the called function gives back may get values from a later caller. Walk the type of the called function, also its own need: its parameter is given to it, its result is given by it, and every type variable in its own need is given to it, and the direction turns at each parameter of a function type and at each contravariant argument of a constructor; an argument of a constructor that is invariant or bivariant, and a need, count as both directions. A type variable at a place given to the called function, other than a place in the parameter reached from its top only through parts of data and covariant arguments of constructors (the argument shows the values there), is matched as usual, and then its type is made whole, both in the expected type of every item and in the known type of the call: every `Int` and `String` member, also in a part of data or an argument of a constructor, or in the result of a function type, has all its values, `Never` there is `_`, and every other part that is not covariant (a parameter of a function type, an argument of a constructor that is not covariant, a need) is `_`. So in `list.fold(xs, 0, (n, x) { … })`, `n` has the known type `Int`, not `0`; with `(0, 0)` as the start, `(Int, Int)`; with `[]` as the start, `List(_)`. With `loop: <S>(S, (S, (S) -> Nil) -> Nil) -> Nil`, in `loop(0, (n, again) { … })`, `n` has the known type `Int`. With `make: <S>(S) -> ((S) -> S) -> S`, `make(0)` has the known type `((Int) -> Int) -> Int`. With `cur: <S>(S) <- (State(S)) -> S`, `cur(0)` has the known type `Int`, so `cur(0) == 5` is correct.
  - The value of a binding whose pattern has `x: T` at its top has `T`. The body of a function with a written `->` part has that type.
  - The last expression of a block has the expected type of the block, and every branch body of a match has the expected type of the match. A part of data or of a list has the matching part of the expected type, when that type has one member, data of that shape or a list.
  - When a function expression has an expected type with one member, a function type whose parameter type has a member of the function's shape, every item of its parameter group with no type written at its top is fitted to the matching part of that member, as for names in patterns below. This only gives known types. So `list.map(users, (u) { u.name })` is correct when `users` has a known type.
  - In a match `s |> (p) { e }` whose chain has one branch with no guard and no alternatives, when `p` has `x: T` at its top, `s` has the expected type `T`, as the value of a binding `x: T = s` does.
  - A name with a selection table used other than as a callee, at a place whose expected type has exactly one member, a function type with parameter part `P`, has the type of the branches selected for each member of `P` whose shape it has (§6.12). So in `list.map([1, 2], show)`, the expected type of `show` is `(Int) -> B`, and the result is selected by it.
- `v.f` and `v.k` have the part type of rule 13, when it has no type variable that is not put in.
- A block has the known type of its last expression.
- In a match `e |> C`, every parameter pattern of the one-position group of `C` is a pattern fitted to a value with the known type of `e` (below), also when the group has one branch and no guard. So with `opt: Option(User)`, `opt |> (Some(u)) { u.name }` is correct, as `Some(u) = opt` followed by `u.name` is.
- **Names in patterns**: when a pattern `q` is fitted to a value with a known type `B`: a catch-all gets `B`, less the members that rule 12 takes out. If `q` is not a catch-all, take the one member of `B` with the type constructor of `Q`; for a literal pattern with the value `v`, only that value, if the member has it: `B & v`, with `v` the literal type of the value (not `B & Q`, because where a pattern is fitted `Q` is the whole `Int` or `String`); if there is not exactly one, the names of `q` get no known type. The parts of `q` then have the known part types of that member, with its type arguments put in; the binder `r` of a string prefix, suffix, or combined pattern has the known type `String`, all its values, because the known type of the whole value says nothing about its remaining part. A name part of a conjunction gets that member. The name `x` of `x: T` gets `T`, whatever the value.

So `(u) { u.name }` needs a type, `(u: User) { u.name }`, but with `opt: Option(User)`, `opt |> (Some(u)) { u.name } | (None) { "" }` is correct.

## 5. Names and scope

- Built-in names (§12) may be used in every module. Imported names and top-level names may cover them.
- A top-level name may be used anywhere in its module (§2.5).
- A local name may be used from the next statement to the end of its block. When the value of its binding is a direct value, it may also be used inside the function bodies in that value (§6.10). The names of a parameter group, and of a service pattern, may be used in the guard and the body of their branch. `( )` makes no scope: the items of data are worked out in the scope around it, and a label is not a name.
- A type parameter of a function (§4.7) may be used in that function, or in the rest of that function type. A type parameter of a type definition may be used in that definition (§4.3). An alias pattern has no type parameters; its binders are types of the module.
- A local name may cover an outer name with the same name, and an earlier binding in the same block.
- A `discard` makes no name.



## 6. Expressions

### 6.1 Grammar

```
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
Binding   = Pattern "=" Expr .
Branches  = Function { "|" Function } .
Function  = [ TypeParams ] Params [ "<-" SvcPat ] [ "->" Union ] Block .
Params    = "(" [ PItem { "," PItem } [ "," ] ] [ "?" Expr ] ")" .
SvcPat    = "(" [ SvcItem { "," SvcItem } [ "," ] ] ")" .
SvcItem   = name ":" NamedType | ".." name ":" UpName .
```

`TypeParams` and `Union` are the productions of §3; `PItem` is that of §7.1. Every rule below decides by tokens and brackets, never by names or types, and none needs a comma.

- **Function or data.** A `(` that begins a primary is the `Params` of a function when its matching `)` is followed by `->`, or by `{` on the same line, or by `<-` and a bracket group whose matching `)` is followed on the same line by `->` or `{`; that group is then the service pattern. Otherwise the `(` begins `Data`, and a `<-` after the data is a provide (§9.2). A primary that begins with `<` is a function with type parameters, and its `>` must be followed by `(`. The decision is made before the contents of the brackets are read. Function bodies are always blocks.
- **Calls.** A `Data` on the same line right after a postfix expression is a call (§1.7). So `f(x) { … }` is a call and then a block.
- **Items.** In `Data`, a data type, a data pattern and `Params`, an item whose first two tokens are a `name` and `=` is a labelled item. `==` is one token, so `(a == b)` has one position. `..` begins a spread or a rest.
- **Branches.** After the `}` of a function body, a `|`, also at the start of the next line, joins the next branch, which must begin a function (`(`, or the `<` of type parameters); anything else after it is a compile error. A `|` after any other expression is a compile error. A call, `.f` or `.k` after a chain applies to the whole chain.
- **The `->` part.** In the `->` part of a function, a `{` right after `->`, `|`, `&` or `-` is part of the type (§3); the first other `{` not inside brackets is the body.
- **Statements.** A statement reads pattern parts while each is followed by `=`. If at least one is, the statement is a binding: the parts read form its pattern, and its value begins at the first part not followed by `=`. Otherwise the statement is an expression. A lowercase name followed by `(` is never a pattern part (§7.1), so `id(A) = …` is an expression followed by a stray `=`, a compile error.
- **Guards.** `?` inside `Params` begins the guard, after the last item, and the guard ends at the matching `)`. A `?` in any other pattern is a compile error.

### 6.2 Order of evaluation

i is strict. Expressions are worked out from left to right:

- The items of data, lists and bit arrays are worked out in the order written; a spread `..e` is worked out where it is written. The order of keys never changes this order.
- In a call, first the function, then the items of the argument data in the order written, then the call.
- In a binary operation, first the left side, then the right side. `&&` and `||` do not do the right side when the left side is enough.
- `a |> f(b)` does `a` first, and then the rest in the order of `f(a, b)`.
- In `e <- s`, `s` is worked out first, in the services around it, and then `e` with the services provided (§9.2).

### 6.3 Literals and names

The type of an integer literal is its literal type (§4.2): `1` has the type `1`. When the operand of `-` is an integer literal, also inside groups, the result has the literal type of the negative number: `-1` and `-{1}` have the type `-1`. Every other `-e` has the type `Int`, as `-x` and `-{-1}`. The type of a string literal is its literal type: `"a"` has the type `"a"`. The type of a float literal is `Float`. The value of a name is the value it is bound to. A member of a module is `module.name`, or an imported name (§2.3). A constructor whose data is `()` is a value; any other constructor is a function from its data (§6.7).

### 6.4 Data

`( … )` in an expression makes data (§4.1). Its items are `e`, the next position, counting from 0; `name = e`, a label; and `..e`, a spread. The type of the data is the data type with the type of each item at its key. Positions come before labels, and a label may not be given twice.

- A spread `..e` needs a known type of `e` (§4.8) with exactly one member, which is a data type, or a labelled type whose constructor may be used here (§2.4). Its positions come next in the numbering, and its labels are added. A labelled item written after a spread may replace a label of that spread; any other repeated key is a compile error.
- `( )` is not a scope: the items are worked out in the scope around the data, in the order written, and a label makes no name. So `(w = h, h = w)` swaps the outer `w` and `h`.
- `()` is the empty data. `(e)` is data with one position; it is not `e`. `{ e }` puts an expression in a group (§6.10).
- `d.k`, where `k` is a decimal, reads position `k`; `d.f` reads the label `f` (§4.8 rule 13).

### 6.5 Lists

The type of `[e1, …, en]` is `List(T)`, where every part has the type `T`. `T` may be a union: `[1, "a"]` has the type `List(1 | "a")`, and may be used where a `List(Int | String)` is needed. `[e1, …, en, ..rest]` puts the parts in front of the list `rest`.

### 6.6 Calls

`f(…)` calls the function `f` with the data in the brackets. Every function is called in this way, whatever expression gives it. The data must have a shape that `f` takes (§4.6). There are no labels outside data and no default parameters: every key is given exactly once, by the data. `f()` passes `()`, and `f(x)` passes the one-position data `(x)`.

### 6.7 Constructors and keys

`C(…)` makes a value of the labelled type `C` (§4.3). It is a call of the constructor with the data in the brackets, which must be in the data type of `C`. `C` alone is the value of a labelled type whose data is `()`; `C()` is a compile error. `C(..v, label = e)` is an ordinary call whose data has a spread.

`v.f` and `v.k` read a key of data, or of the data of a labelled value. Every member of the known type of `v` (§4.8) must have that key, and its type is the union of those parts (§4.8 rule 13). With no known type it is a compile error. Reading a key of a labelled value, and spreading it, needs the same visibility as taking it apart with a constructor pattern (§2.4).

### 6.8 Functions

`<A, N>(p1, …) <- (x: T, …) -> R { body }` is a function value. Every part but the parameter group and the body may be left out.

- The **parameter group** `( … )` is a data pattern (§6.11). Its shape is the shape of the function.
- The **service pattern** after `<-` names the services that the function itself uses: each item `x: T` binds `x` to the service whose key is the named type `T` (§9.2), and the type is required. `..env: N` binds the whole set of services that the function receives, as one value of the row type `N`; it is the last item, at most once, and `N` must be a type parameter of the function. A service pattern has no labels, no guard and no alternatives, and is not part of the shape. `<- ()` is a compile error: a function that takes no service writes no `<-` part.
- The **`->` part** is the type of the value the function gives. When it is left out, the value is inferred.
- **Type parameters** `<A, N>` are in scope in the whole function (§4.7).
- The function may use local names from outside. The body is a block, and the value of the block is the value that the function gives back. The need of the function is inferred (§4.8 rule 4), also when the `->` part is written.

### 6.9 Operators

Operators are part of the syntax. They are not functions, and they may not be used as values. They are in this order, from weakest to strongest. Operators at the same level go from left to right.

| Level | Operators | Operand type | Result type |
|---|---|---|---|
| 0 | `<-` | See below | |
| 1 | `\|>` | See below | |
| 2 | `\|\|` | Bool | Bool |
| 3 | `&&` | Bool | Bool |
| 4 | `==` `!=` | Any two types; see below | Bool |
| 5 | `<` `<=` `>` `>=` | Int | Bool |
| 5 | `<.` `<=.` `>.` `>=.` | Float | Bool |
| 6 | `<>` | String | String |
| 7 | `+` `-` | Int | Int |
| 7 | `+.` `-.` | Float | Float |
| 8 | `*` `/` `%` | Int | Int |
| 8 | `*.` `/.` | Float | Float |

The operators take values of literal types, because these are subtypes of `Int` and `String`. Their results have the whole type: `1 + 2` has the type `Int`, not `3`, and `"a" <> "b"` has the type `String`. The one exception is `-` on an integer literal, also inside groups, which has a literal type (§6.3): `-1` and `-{1}` have the type `-1`, but `-x` has the type `Int`.

The unary operators `-` (Int) and `-.` (Float) are stronger than all binary operators. Calls, `.f`, and `.k` are stronger still. There is no operator for "not"; the function `core.not` (§13) does this.

- Int and Float have different operators. One is never changed into the other without a function such as `core.to_float` (§13).
- `a |> f` is `f(a)`, passing the one-position data `(a)`. `a |> f(b, port = 1)` is `f(a, b, port = 1)`: `a` is put in as position 0. `a |> f(..t)` is `f(a, ..t)`. The right side of `|>` must be a name, a call, or a chain (§6.12); anything else is a compile error that suggests a group. `|>` is weaker than every operator but `<-`, so its whole left side is its left operand: `a == b |> f` is `{ a == b } |> f`, and `x |> g == 2` is a compile error.
- `e |> C`, where `C` is a chain, is a match (§6.12).
- `e <- s` works out `e` with the services `s` provided (§9.2). Its left operand is the whole expression to its left, and its right operand is one postfix expression: `a + f(x) <- (s)` provides to `a + f(x)`, `x |> g <- (s)` provides to `x |> g`, and `e <- (s) + 1` is a compile error. A group provides to a part: `a + { f(x) <- (s) }`.
- When both sides of `==` or `!=` have known types (§4.8), and these are disjoint (§4.5 rule 3), the compiler gives warning W0503, showing both types: the two sides can never be equal. The comparison is well defined and has type `Bool`: `==` is always `False` and `!=` is always `True`. A member of a known type that is `_` or a type variable is never disjoint from anything. Literal types count. So `1 == "a"`, `1 == 2`, `1 != 2`, `"a" == "b"`, `x == "b"` after `x = "a"`, and `(s: String) { s == 1 }` give the warning, but `(s: String) { s == "a" }`, `(m: "a" | "b") { m == "b" }`, `x == Nil` with `x: Int | Nil`, `[1] == []`, `a == b` with a type variable, and `(s) { s == 1 }` do not. Only the top members of the two types are compared (§4.5 rule 3), so `(1, 2) == (3, 4)` and `Some(1) == Some(2)` give no warning, though they are always `False`. Literal types are not widened. The warning adds no narrowing fact. A guard `? x == 2` with `x: 1` is always false; coverage treats it as any guard (§7.4). What `==` does is in §11.4.

### 6.10 Blocks and bindings

The statements of a block are done in order. The value of a block is the value of its last expression. The last statement may not be a binding. A block with only one expression puts it in a group: `{ a + b } * c`. A block is the only thing that makes a scope.

An expression statement that is not the last, and whose working out can do nothing but make its value, gives a compile warning: its value is not used. These are literals, names, functions, chains, and data, lists, constructor calls, `.f`, `.k`, `-`, and Int, Bool, String, and comparison operators made only of these. A call, `|>`, a provide, a block, a bit array, and a Float operator, which may stop (§11.3), do not give the warning. So `assert 1 + 1 == 3`, which is the two statements `assert` and `1 + 1 == 3` (§1.7), gives two warnings.

`p = e` is a binding. It works out `e` and matches it with the pattern `p`. The names in `p` may be used from the next statement. `e` is a whole expression, so `u = find_user(id) <- (db)` binds the result of the provide.

- The type of `e` must be a subtype of the pattern type of `p` (§4.8 rule 10). So `x: Int = e` says that `e` has the type `Int`, and `(a, b) = v` with `v: (Int, Int) | Nil` is a compile error; a match is used for this.
- If `p` can not fail for the type of `e` (§7.3), the binding needs no service.
- If not, as in `Ok(v) = r`, when it runs, the binding works like the match `e |> (p) { … } | (_) { a.fail(AssertionFailure(message = m)) }`, where the first branch goes on with the rest of the block. The binding is still typed by §4.8 rule 10, not by rule 11. Here `a` is the `Assert` service, and `m` is the text `pattern `, then the source text of `p`, then ` did not match `, then the text form (§11.8) of the value. So `Assert` is in the need of the binding (§9.6).
- A pattern is a conjunction of parts, `p1 = … = pk`: `a = (x, y) = e` binds `e` to `a` and to `(x, y)`, and `a = b = c = d` binds `a`, `b` and `c` to `d`.

A **direct value** is an expression that is made without running any code: a function expression or a chain; a literal, also with `-` or `-.` before it, also when the literal is inside groups (`-{1}`, `-.{1.5}`); a name; or data with no spread, a list, a constructor call, or a group `{ e }` whose parts are direct values. When the value of a binding is a direct value, the names of the binding may also be used inside the function bodies in that value. This is the same for every binding, local or top-level. So a local function may call itself, and one binding of data may make functions that call one another:

```i
fact = (0) { 1 } | (n: Int) { n * fact(n - 1) }

(even, odd) = ((n) { n == 0 || odd(n - 1) }, (n) { n != 0 && even(n - 1) })
```

### 6.11 Parameters

A function's parameter group is a data pattern (§7). When the function is called, the argument data is matched with it as in a binding (§6.10), but its names are not generalized (§4.7): the names of the pattern may be used in the guard and the body, and a pattern that may fail reports to `Assert`.

- An item `p` is a positional item; an item `label = p` matches the part under `label`. `(name: T)` is a positional item that binds `name`; labels are given only by `label = p`.
- The parameter group has no rest `..` at its top: the shape of a function is fixed by its written group. Rest is allowed in nested data patterns.
- `(port = 80)` in a parameter group says that the label `port` must have the value `80`; there are no default parameters. Optional parts are written as branches of different shapes (§6.12).
- The names of one parameter group may not be bound twice, and its labels are distinct.
- A group may end with a guard `? e` (§6.12).

### 6.12 Branches and matching

A **chain** `F1 | … | Fn` of function expressions, with `n ≥ 1`, is one function value. Each branch may write its own type parameters, which are in scope in that branch only. The branches are grouped by shape: a **shape group** has the branches whose parameter groups have one shape, in the order written. When the chain is called with data, the group with the shape of the data is used, and in it the first branch whose parameter group matches the data and whose guard, if it has one, is `True` is taken; the value of its body is the value of the call. A guard is any expression of type `Bool`. It is worked out only after its parameter group matched, and it may use the names of the group and of the service pattern. Alternatives in an item are tried in order.

```i
gcd = (a: Int, 0) -> Int { a } | (a: Int, b: Int) -> Int { gcd(b, a % b) }

area = (r: Float) -> Float { 3.14159 *. r *. r } | (w: Float, h: Float) -> Float { w *. h }

label = (opt) {
  opt |> (Some(v) ? string.is_empty(v)) { "empty" }
      | (Some(v)) { v }
      | (None) { "none" }
}
```

- A group with one branch and no guard is a function of one shape: its parameter group is fitted (§4.8), `x: T` in it is a type annotation unless its item has alternatives, it is not checked for coverage, and when it may fail it reports to `Assert`.
- Any other group is typed by §4.8 rule 11, and its branches must cover every value of its parameter type (§7.4). If not, it is a compile error. A guarded branch is not counted for coverage.
- A chain whose groups all have one shape has the type `(B) <- N -> R`, where `R` is the union of its branch results. A chain with several groups has the type `{ B1 | … | Bm } <- N -> R`, where `Bi` is the parameter type of group `i` and `R` the union of the results of all branches. All branches of a chain share one need `N`, the merge of the needs of their guards and bodies and of their service patterns. Calling such a value with data of shape `S` uses the group of shape `S`, at run time also when the value went through data, a list or another function.
- `e |> C`, where `C` is a chain, is a **match**, and `e` is its **subject**. It works out `e` and calls `C` with `(e)`, so it uses the group of shape "one position". A match adds no call frame: the branches run in the place of the match, a call in a branch body of a match in tail position is in tail position (§11.6), and the match is not one of the calls in a stop message (§11.7). Branches of other shapes in a matched chain can never run. To match a pair, write a pattern for it in one position: `p |> ((0, y)) { y } | ((x, _)) { x }`.
- A branch that can never be used gives a compile warning.
- A chain is a direct value (§6.10), as every function is. Where §2.5, §5, and §6.10 speak of the function bodies in a value, the guards and branch bodies of a chain count as function bodies. Each time a chain is worked out, it makes a new value, which is the same only as itself (§11.4).
- `core.never : <A>(Never) -> A` (§13) is the match on `Never`: `core.never(x)` is correct only when `x` has no values.

**Selection.** A binding whose pattern is one name with no written type, and whose value is a chain or a name of a binding with a selection table, after removing groups, has a **selection table**: the branches of that chain, each with its shape, its written parameter pattern, its parameter type and its result type. A chain bound with a written type, put in data or a list, or passed as an argument, has no table. A call `f(d)`, `x |> f` or `x |> f(…)` of a name `f` with a table is typed by the branches it **selects**:

1. The shape of the argument data is read from the call: its positions and labels, and the keys of each spread, which need a known shape. No group of that shape is a compile error that lists the shapes `f` has.
2. In that group, a branch is skipped when, at some key, the known type of that item of the data is disjoint (§4.5 rule 3) from the branch's pattern type at that key less what earlier unguarded branches of the group take there (§4.8 rule 12, applied at every place). An item with no known type is never disjoint from anything. Every branch that is not skipped is selected. When none is selected, it is a compile error that names the known types of the items.
3. The call is typed as a call of the function type whose parameter is the union of the parameter types of the selected branches and whose result is the union of their results.

So with `show = (n: Int) -> String { int.to_string(n) } | (s: String) -> Int { string.length(s) }`, `show(1)` has the type `String`, `show(x)` with `x: Int | String` has the type `String | Int`, and `h = (x) { show(x) }` gives `h` the type `(Int | String) -> String | Int`. A name used where a function type is expected selects for that type (§4.8), so `list.map([1, 2], show)` is a `List(String)`. Selection chooses only a type; at run time the first matching branch is always taken.

## 7. Patterns

### 7.1 Grammar

```
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

- A `Name` in a pattern must be a constructor (an `UpName`, or `m.C`). Its data is matched by a data pattern `C(…)`, with every key of its data type or `..` at the end. `C` alone matches a labelled value whose data is `()`; `C()` is a compile error. A lowercase name followed by `(` is not a pattern.
- A data pattern `( … )` matches data of its shape whose parts match its items. A positional item matches the next position, and `label = p` matches the part under `label`. A data pattern lists every key of its shape, or ends with `..`, which leaves out the other keys, or `..r`, which binds them as data: the remaining positions numbered again from 0 in order, and the remaining labels. `..` needs the shape of the matched value to be known, from a type or from the constructor. A positional item may not follow a labelled item.
- A pattern is a conjunction `p1 = … = pk`: it matches what every part matches, and each part binds its names. Directly inside a data pattern or a parameter group, an item whose first two tokens are a `name` and `=` is a labelled item, never a conjunction. So to name a whole positional item, the name is written last: `((a, b) = args)`. `(size = whole = (w, h))` is the label `size`, whose part matches both `whole` and `(w, h)`. A positional item that is a conjunction of names starts with a discard: `(_ = x = y)`.
- A labelled part of a constructor's data is matched only by its label: with `User = (name = String, age = Int)`, `User(name = n, age = a)` and `User(age = a, ..)` are correct, and `User(name, age)` is a shape error. There is no shorthand for `name = name`.
- Alternatives `p1 | p2` are allowed only in the items of a parameter group (§6.11). Every alternative binds the same names, and a name gets the union of its types. `|` is looser than `=`.
- The type after `:` goes as far to the right as it can, as every type does (§3). So `x: Int | String` tests for a union, and `f: (Int) -> Int = e` needs no brackets. In a parameter item, a type ends at `,`, `?` or `)`. A `|` after a type is part of the type, so a type test is written after the other alternatives: `(True | _: Int)`. The type is read with no going back. In `(x: Int | y)`, `Int | y` is a type, and `y` is a value type (§3).

`x: T` is one thing in every place where it may be written (in a bit array segment, `:` begins a format instead, §8.1). It matches the values of the type `T`, and `x` gets the type `T`. In a binding, and in a parameter item that is fitted (§4.8) and has no alternatives, every value that comes there is in `T` (§4.8 rules 4 and 10), so `x: Int = e` and `(x: Int) { … }` are type annotations. In a group typed by §4.8 rule 11, and in alternatives, `x: T` tests the value. At run time, a member of `T` is tested by its type constructor; a member of `Int` or `String` whose value set is not all the values is also tested by equality (§11.4) with the values in it. So `x: "a" | "b"` takes the strings `"a"` and `"b"`, and `x: String - "a"` takes every other string, but no Int. So in a test, `T` may not have a type variable as a member. Opaque types cannot be told apart at run time, so for a type test they all count as one type constructor, as all function types do.

### 7.2 What patterns match

| Pattern | Matches | Names |
|---|---|---|
| `x` | Any value | `x` |
| A discard | Any value | None |
| A literal, or `-` or `-.` and a literal | An equal value (§11.4) | None |
| `C`, `C(…)` | A value made by `C` whose data matches the data pattern | Those of the items |
| A data pattern | Data of its shape, or of a shape with more keys when it ends with `..`, whose parts match the items | Those of the items, and the rest |
| `[p1, …, pn]` | A list of exactly n parts that match | Those of the parts |
| `[p1, …, pn, ..r]` | A list of n or more parts; `r` is the rest | Those of the parts, and `r` |
| `"lit" <> r` | A string that starts with `"lit"`; `r` is the rest | `r` |
| `r <> "lit"` | A string that ends with `"lit"`; `r` is the start | `r` |
| `"a" <> r <> "b"` | A string with that prefix and suffix, without overlap; `r` is the middle | `r` |
| `p1 = … = pk` | What every part matches | Those of every part |
| `x: T` | A value whose type constructor is that of a member of `T`; for a member of `Int` or `String`, the value must also be in the value set of the member | `x` |
| `<< … >>` | See §8 | |

- A string pattern has exactly one binder (or discard), whose inferred and known type is `String` (§4.8). Its literals have no interpolation. `a <> "x" <> b` is a compile error. Prefix and suffix must not overlap: `"ab" <> r <> "ba"` does not match `"aba"`.
- A pattern may not bind one name two times.

### 7.3 Patterns that may fail

A pattern **can not fail** for a type `T` when it matches every value of `T`. Other patterns **may fail**. A catch-all can not fail for any type, and `x: U` can not fail for a type `T` with `T <: U`, so `id = <A>(x: A) -> A { x }` needs no `Assert`. A value of a type variable may be any value, so a pattern that tests more than its type, such as a literal or a constructor, may fail for it.

A literal pattern can not fail for its literal type, so `"a" = x` with `x: "a"` needs no `Assert`. Where a pattern is fitted (§4.8), a literal has the whole `Int` or `String`, so `"a" = s` with `s: String` is correct and may fail: it reports to `Assert`. A constructor pattern fitted to a union, as `Ok(v) = r` with `r: Ok(Int) | Error(String)`, is correct and may fail. Whether a pattern may fail is decided with every value that may come, also when the value is a direct value of the same group (§6.10), whose values are known only when the whole group is checked: in `pair = ("b", () { ("a", _) = pair  1 })`, the binding may fail, and the function reports to `Assert`. The same holds for a match whose chain has one branch and no guard.

### 7.4 Covering every value

The branches of a shape group whose parameter has the type `A` **cover** `A` when, for every value of `A`, some branch without a guard has a parameter group that matches it. A branch with a guard is not counted. A branch is **never used** when every value that its pattern matches is matched by an earlier branch without a guard.

A literal pattern matches its own value, and string prefix/suffix patterns match the strings with their fixed ends. So literal patterns cover an `Int` or `String` member whose value set is finite, when every value in it is matched: with `m: "GET" | "POST"`, `m |> ("GET") { 1 } | ("POST") { 2 }` is covered. Literal patterns alone never cover a member with all its values, or with all but a finite set; a catch-all, a type test such as `x: String - "a"`, or, for `String`, the patterns `"" <> r` and `r <> ""`, which match every string, must take the other values. A literal in an item of a list pattern, or at a key of a constructor whose type parameter occurs more than once in its data (`Twin(A) = (A, A)`), has the whole `Int` or `String` (§4.8), so such literals cover a finite set only when that type is written or comes from the subject: `(xs: List("a" | "b")) { xs |> ([]) { 0 } | (["a", ..]) { 1 } | (["b", ..]) { 2 } }` is covered, and the same chain with `(xs)` is not. Literal patterns of Float and BitArray never cover their type. A union is covered when every member is covered. Data of one shape is one type constructor.

When the values that are not covered are only values of `Int` or `String` that no literal pattern takes, the type of the group already says so (§4.8 rule 11: `(n: Int) { n |> (0) { "zero" } | (1) { "one" } }` needs `Int <: 0 | 1`). In a match whose subject already has such a type, that is reported as this error alone, with a value that no branch matches, such as `2`. A chain that is not the right side of a match gets these values later, through its parameter, so there it is a type error where the values come and also this error (§4.8 rule 11). When a value that is not covered is at a list item place, or at such a key, where the branches have only literals, the message also says to write that type, such as `List("a" | "b")` or `Twin("a" | "b")`.

Only a group with more than one branch or with a guard must cover the type of its parameter (§6.12). A group with one branch and no guard is not checked for coverage: when its pattern may fail, it reports to `Assert`.

### 7.5 Known constructors

When a conjunction with a name part `x` and a part `C(…)` matches, `x` is known to be made by `C` where `x` may be used. When a branch pattern `C(…)` of the match `v |> C1 | …` matches the value of the name `v`, and its group has more than one branch or a guard (§4.8 rule 11), `v` is known to be made by `C` inside that branch. A match with one branch and no guard gives only what the binding of its pattern to `v` gives (§6.12). This is used by §6.7.


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

i has no special effect definitions. Effects are made from normal things: functions with no parameters (§9.1), services (§9.2), errors as the service `Fail` (§9.3), and the control operations of `i/control` (§9.4). There is no error channel, no implicit capability and no layer concept.

### 9.1 Computations

A function with no parameters, `() <- N -> A`, is a **computation** that has not been done yet. It is the same idea as an `Effect<A, E, R>` value in Effect-TS, which is `() <- (..R, Fail(E)) -> A` in i. It may be stored, given to other functions, and put together with other computations. Calling it does it. Retrying, time limits, and running at the same time are normal functions:

```i
retry = <A, E, N>(times: Int, body: () <- (..N, Fail(E)) -> A) -> A {
  fail.catch(body) |> (Ok(value)) { value }
    | (Error(error) ? times > 1) { retry(times - 1, body) }
    | (Error(error)) { fail.raise(error) }
}

user = () { retry(3, () { load_user("1") }) }
```

### 9.2 Services

A **service** is an ordinary value of a labelled type (§4.3), normally one whose data holds functions. Two services of one form are made by defining two types. A service is a normal value: it may be stored and given to functions, and code that has it may call its functions directly.

The **need** `N` of a function type `(P) <- N -> R` is the set of services that a call takes from the place where it is called. Every item of a need, of a service pattern and of provided data is **keyed** by the type constructor of its type: order does not count, and there are no names or labels. `<- (Db, Log)` is the same type as `<- (Log, Db)`. Needs are shown in the UTF-8 byte order of the qualified names of their type constructors, then the row variable. Only after `<-` is anything keyed by type; ordinary data keeps positions and labels.

```i
Clock = (now = () -> Int)

elapsed = (start: Int) <- (clock: Clock) { clock.now() - start }
// elapsed : (Int) <- (Clock) -> Int

fixed = () { elapsed(40) <- (Clock(now = () { 42 })) }
// fixed : () -> Int
```

- **Taking.** The service pattern of a function head (§6.8) names the services that the function uses itself: `x: T` binds `x` to the service keyed by `T`. A need that a head does not write still passes down: a function that calls `elapsed` needs `Clock` too, and inference writes it into its type.
- **Calls.** The services go with every call: a called function gets the services of the place of the call. Nothing marks such a call. So a function made in one place and called in another gets its services from the place where it is called.
- **Providing.** `e <- s` works out `e` with the services of the place plus the data `s` (§6.9), for any expression `e`; `s` is worked out first, with the services of the place. For a key that `s` has, `s` wins; the other keys come from outside. `f(x) <- (s)` provides to one call, and `{ … } <- (s)` to every statement and the result of a block. Provided data is a data literal whose items are expressions or spreads, with no labels, or any postfix expression whose type is a data type with positions only or a row type (§9.4). An explicit item wins over an item from a spread with the same key; two explicit items, or two spreads, with one key are a compile error. A provided item is keyed by its known type (§4.8), which must be one named service type: not a union, a type variable, or unknown. A provided key that the need does not have gives a warning.
- **Merging.** When a need has one type constructor twice, the two entries are merged, parameter by parameter, by the direction of the parameter (§4.5 rule 6): a parameter that goes in the other direction gives the union, as `Fail(A)` and `Fail(B)` give `Fail(A | B)`; one that goes in the same direction gives the intersection, and a compile error when that is `Never` while neither is; one that does not change must be equal, else it is a compile error. A provided value of type `P` satisfies the need entry `S` when `P <: S`, so a `Fail(A | B)` satisfies code that needs only `Fail(A)`. An entry that every value of its type constructor satisfies, as `Fail(Never)`, is removed.
- **Row variables.** A type parameter can stand for a whole need: `map : <A, B, N>(List(A), (A) <- N -> B) <- N -> List(B)`. In `(..N, C(…))`, `N` has no entry for `C`. A rigid row variable that reaches a provide of `C` without a stated `C` entry is a compile error.
- **Branches.** All branches of a chain share one need (§6.12).
- Type checking makes certain that every service is there when a call takes it. Getting it never fails at run time.

`Await`, `Root` and `Owner` (§9.11), `Scheduler` and `Runner` (§9.9) are normal service values. Replacing a host service in i is a provide, as for any service. Replacing a random seed with `task.top` keeps the run route of the current `Runner`.

A service value can be captured. Providing the service later changes calls that take it from their caller, not the captured value:

```i
Clock = (now = () -> Int)
Reading = (read = () -> Int)
make = () <- (clock: Clock) { Reading(read = () { clock.now() }) }
now = () <- (clock: Clock) { clock.now() }
example = () {
  saved = make() <- (Clock(now = () { 1 }))
  (saved.read(), now()) <- (Clock(now = () { 2 }))
}
// example() gives (1, 2).
```

**Layers.** There is no layer concept. A function that returns several services returns ordinary data, which may be provided whole or spread:

```i future-SYN3
base = () <- (io: Io) -> (Config, Log) {
  (Config(url = "https://example.com"), Log(write = (s) { io.print(s) }))
}
live = () -> (Config, Log, Db) {
  s = base()
  d = db_layer() <- s
  (..s, ..d)
}
main = () { app() <- live() }
check = () { app() <- (..live(), fake_db) }
```

Here `live` has the type `() <- (Io) -> (Config, Log, Db)`, and in `check` the explicit `fake_db` wins over the `Db` of the spread. The order in which services are made is written by hand.

### 9.3 Errors

An error is raised through the prelude service `Fail(E)`:

```i
Fail(E) = (raise = (E) -> Never)
```

An error may be a value of any type. Needs merge by direction (§9.2), so a function that calls functions needing `Fail(A)` and `Fail(B)` needs `Fail(A | B)`. The module `i/fail` has these functions, written in i over `Fail` and `control.escape` (§9.4):

| Function | Type | What it does |
|---|---|---|
| `fail.raise` | `<E>(E) <- (Fail(E)) -> Never` | Ends the computation with the error |
| `fail.catch` | `<A, E, N>(() <- (..N, Fail(E)) -> A) <- N -> Result(A, E)` | Changes every error of the body into `Error(…)` |
| `fail.from_result` | `<A, E>(Result(A, E)) <- (Fail(E)) -> A` | Raises `x` for `Error(x)` |
| `fail.map_error` | `<A, E, E2, N>(() <- (..N, Fail(E)) -> A, (E) <- N -> E2) <- (..N, Fail(E2)) -> A` | Changes the errors |

```i
raise = <E>(error: E) <- (handler: Fail(E)) -> Never { handler.raise(error) }

catch = <A, E, N>(body: () <- (..N, Fail(E)) -> A) -> Result(A, E) {
  control.boundary((b) {
    Ok(body() <- (Fail(raise = (e) { control.escape(b, Error(e)) })))
  })
}
```

Because `N` has no entry for `Fail`, `catch` takes every error of its body.

```i
load = (id) { parse_user(http_get(id)) }
// load : (String) <- (Fail(HttpError | ParseError), Http) -> User

safe = (id) {
  fail.catch(() { load(id) }) |> (Ok(user)) { user }
    | (Error(_: ParseError)) { default_user }
    | (Error(other)) { fail.raise(other) }        // other: HttpError
}
// safe : (String) <- (Fail(HttpError), Http) -> User
```

- To handle only some errors, take all of them with `fail.catch`, then match with type tests. A later catch-all gets the rest of the union (§4.8 rule 12), which may be raised again, as in `safe` above.
- An error goes out through the calls to the `catch` that provided its `Fail`. The computation that is jumped over is unwound first (§9.5).
- A `Fail` value that is used after the `catch` that made it has ended, because it was returned, stored, or captured by a function that runs later, stops the program with the message `boundary not active` (§9.4). The compiler warns when a `Fail` bound in a service pattern is captured by a function value or put into data.
- `Assert` is a separate service (§9.6), so `catch` does not take assertion failures.
- At an entry, the platform provides a top-level `Fail` (§10.2): an error that reaches it from the program entry or a test stops the program or fails the test, and one that reaches it from a host export is given to the host.

### 9.4 Control

The module `i/control` pauses a computation and goes on with it later:

| Function | Type |
|---|---|
| `control.boundary` | `<R, N>((Boundary(R, N)) <- N -> R) <- N -> R` |
| `control.suspend` | `<A, R, N>(Boundary(R, N), (Continuation(A, R, N)) <- N -> R) -> A` |
| `control.resume` | `<A, R, N>(Continuation(A, R, N), A) <- N -> R` |
| `control.cancel` | `<A, R, N>(Continuation(A, R, N)) <- N -> Nil` |
| `control.escape` | `<R, N>(Boundary(R, N), R) -> Never` |

`Boundary` and `Continuation` are built-in types. Their values are made only by `control.boundary` and `control.suspend`, and the host cannot take them (§10.3).

1. `boundary(body)` makes a boundary `b` and does `body(b)`. The value of `body` is the value of `boundary`. `b` is **active** while the computation between it and the running code is on the stack: from the start of `body` until `body` ends, but not while that part is paused in a continuation. After `resume`, it is active again.
2. `suspend(b, f)` pauses the computation from this point back to the boundary `b`. That paused part is a **continuation** `k`. Then `f(k)` is done in the place of the boundary, with the services of that place, and its value is the value of `boundary`. If `b` is not active, the program stops.
3. `resume(k, v)` goes on with the paused computation, with `v` as the value of `suspend`. It goes on on top of the place where `resume` is called, and the value that the computation has when it gets to the boundary is the value of `resume`. The paused part keeps the services provided inside it; for the rest, it gets the services of the place where `resume` is called. So an error raised after `resume` goes to the `Fail` of the place of `resume`, unless the paused part provided its own. The services that a function made by `keep` keeps (below) count, for the code inside that function, like services provided inside the paused part.
4. `cancel(k)` unwinds the paused computation (§9.5) and gives `Nil`. While it unwinds, the boundary stays not active, and the final functions get the services as in rule 3, from the place where `cancel` is called; inside a function made by `keep`, they get the kept services.
5. `escape(b, v)` ends the computation back to the boundary `b` at once, unwinding it (§9.5), and `v` is the value of `boundary`. It is `suspend(b, (k) { cancel(k)  v })` with no continuation made. If `b` is not active, the program stops with the message `boundary not active`.
6. **One use only**: `resume` and `cancel` together may be used one time for a continuation. A second use stops the program.
7. If a continuation is never resumed or cancelled, its final functions are not run.
8. A boundary and a continuation stay on the worker that made them (§11.9). They are not portable.

A structured task cancel resumes its scheduling continuation with a cancel answer; i code then cancels an inner continuation (§9.9). It does not change rule 4's rule that the boundary of the cancelled continuation is not active during unwinding.

**Keeping services.** In a service pattern, `..env: N` binds the whole set of services that the function receives as one value of the row type `N` (§6.8). A value of a row type may be bound, captured and provided after `<-`, alone or as a spread in provided data; it may not be spread into ordinary data, compared, printed or matched. `keep` makes a function that, wherever it is called, calls `f` with the services of the place where `keep` was called, but with the caller's `Fail`:

```i future-SYN3
keep = <S, A, E, N>(f: (S) <- (..N, Fail(E)) -> A) <- (..env: N) -> (S) <- (Fail(E)) -> A {
  (x) { f(x) <- env }
}
```

An error raised inside the kept function goes to the `catch` of its caller.

Generators, async code, and schedulers are libraries made from services and these operations. For example, a generator:

```i
Yield(A) = (emit = (A) -> Nil)

collect = <A, N>(body: () <- (..N, Yield(A)) -> Nil) -> List(A) {
  control.boundary((b) {
    body() <- (Yield(emit = (x) { control.suspend(b, (k) { [x, ..control.resume(k, Nil)] }) }))
    []
  })
}
```

### 9.5 Unwinding and final functions

A computation that is dropped is first **unwound**: its final functions are run, from the innermost to the outermost. This takes place when an error or an `escape` goes through it (§9.3, §9.4), and when its continuation is cancelled (§9.4).

The module `i/resource` puts final functions in place. `Scope` is a service of `i/resource`; the platform gives every entry one (§10.2).

| Function | Type | What it does |
|---|---|---|
| `resource.ensure` | `<A, N>(() <- N -> A, () <- N -> Nil) <- N -> A` | Runs the second function after the first, whether the first ends normally, ends with an error, or is unwound |
| `resource.acquire` | `<A, N>(() <- N -> A, (A) -> Nil) <- (..N, Scope) -> A` | Opens a resource and attaches its release to the current `Scope` |
| `resource.scoped` | `<A, N>(() <- (..N, Scope) -> A) <- N -> A` | Runs the body with a new `Scope`, and runs the attached releases in reverse order when the body ends, also by an error, an escape or a cancel |

A release that needs services is made with `keep` (§9.4). A final function may not raise errors. When the program stops (§11.7), final functions are not run.

A structured task group waits for cancelled children and their final functions before it returns (§9.9). Those final functions may suspend and resume and use services, but may not raise errors. An unstarted child has no installed final functions. A host failure or a destroyed instance cannot run i final functions; the host instead releases its own work and resources (Appendix B).

### 9.6 Assert

`Assert` and `AssertionFailure` are prelude types:

```i
AssertionFailure = (message = String)
Assert = (fail = (AssertionFailure) -> Never)
```

An `Assert` service is where failures are reported. Its `fail` never gives back a value. The built-in function `assert` is a normal function, which may be written in i:

```i
assert = (c: Bool) <- (a: Assert) -> Nil {
  c |> (True) { Nil } | (False) { a.fail(AssertionFailure(message = "assert")) }
}
// assert : (Bool) <- (Assert) -> Nil
```

A binding whose pattern may fail also reports to `Assert` (§6.10).

At an entry point (§10.2), the platform gives an `Assert` whose `fail` stops the program; in a test, the test fails. Any code may provide its own `Assert`. The module `i/assertion` has one function that does this:

```i
catch = <A, N>(body: () <- (..N, Assert) -> A) -> Result(A, AssertionFailure) {
  control.boundary((b) {
    Ok(body() <- (Assert(fail = (failure) { control.escape(b, Error(failure)) })))
  })
}
```

`Assert` is for mistakes in the program. A failure that may be expected uses an error (§9.3). `fail.catch` and `fail.map_error` do not take `Assert` failures.

### 9.7 Host services

A type definition with the attribute `@host("namespace")` is a **host service**:

```i
@host("app:storage/v1", "async")
Storage = (read = (String) <- (Fail(String)) -> String)
```

- It is a labelled type whose data has only labels, each with a function type. The parameter data of these functions has positions only. Their parameters and values must be types that the host can take (§10.3). A function type there may have no need, or exactly `<- (Fail(E))` with an `E` that the host can take: the host's error is raised through the caller's `Fail`. Two host services may not have the same namespace and a field with the same label.
- With `@host("namespace", "async")`, the host does the functions **later**: the program waits at the call, and goes on from the call when the host gives a value or an error. In i code, such a call is a normal call.
- A host service that the platform provides at an entry (§10.2) is given by the host. Every field is the Wasm import with the import module `namespace` and the field label as its name, encoded as in Appendix B. i code may also provide a host service, for example to put a fake in its place in a test; then no import is used for it.
- The library module `i/io` declares and exports the **standard service** `@host("i:io/v1") Io = (print = (String) -> Nil)`. `print` writes the text and a line feed to the standard output, and gives `Nil`, also after an output error. A program that prints imports it, for example `((Io) = i/io)`. A host service declared in another module with the namespace `i:io/v1` is a normal host service, and the rule above about namespaces and labels is true for it as for any other: with a field `print`, it is a compile error.

**Targets and platform services.**

The module `i/target` exports these types; the modules `i/js` and `i/wasi` export the roots named in them (§10.1, §B.7.1):

```i
Js = (js = js.Js)
NodeJS = (js = js.Js)
Wasi = (wasi = wasi.Wasi)
Native = ()
Host = ()
Target = (Js | NodeJS | Wasi | Native | Host)
```

`Target` is a service that the platform provides at every entry, holding the target the program was built for. Only §10.1.2's certified entry binding is known to hold the build's target. A program may provide another `Target`.

At an entry the **platform** of the target provides its services: `Assert`, `Scope`, `Fail`, `Root`, `Owner`, `Await`, `Runner`, `Scheduler`, `Target`, and the host services of this table. No library and no package chooses services for the program: every other service is provided in the program's code, normally in `main`. A program built for targets that need different services has one entry for each such target.

| Target | Host services of the platform |
|---|---|
| js | Io, Clock, Entropy |
| nodejs | Io, Clock, Entropy, Arguments, Environment, WorkingDirectory, Fs, Net, Process, Stdio |
| wasi | Io, Clock, Entropy, Arguments, Environment, Fs over preopened directories, Stdio |
| native | Io, Clock, Entropy, Arguments, Environment, WorkingDirectory, Fs, Net, Tls, Process, Stdio |
| host | The services that the embedder registers |

The JS loader checks the required feature profile before starting: WasmGC, tail calls, exception handling, console and clocks/timers, and secure entropy. There is no insecure random fallback. A target never installs a service that only reports `unsupported`. Resource services use the same granted factories in generated Node builds and the SDK; an arbitrary Js value cannot manufacture authority over their private resource numbers.

The process services are split by capability, keeping their operation arguments, results, `ProcessError` values and cancellation rules. Each public wrapper needs only the service in its row, plus `Await` and `Owner` when it waits. `Clock`, `Fs`, `Net`, `Stdio` and `Io` are separate:

| Service | Namespace | Raw fields |
|---|---|---|
| Entropy | `i:entropy/v1` | entropy |
| Arguments | `i:arguments/v1` | args |
| Environment | `i:environment/v1` | env |
| WorkingDirectory | `i:working-directory/v1` | cwd |
| Process | `i:process/v1` | spawn, wait, terminate, close |

The first four have public constructors for ordinary fakes. Process keeps private raw fields and a private labelled `Child` type over Int. The same private Int representation is used for other IO resources. Neither these types nor their errors add a boundary type (§10.3).

`i/net` includes DNS: `connect` accepts a host name or numeric address, and `resolve(name)` returns addresses. The native host uses the system resolver and tries addresses in order, without IPv4/IPv6 racing. TLS belongs to `i/tls`. UDP datagrams are part of `i/net`: `bind(address, port)` returns a private labelled `Datagram` over Int, `send_to(socket, address, port, bytes)` sends one datagram, and `receive_from(socket, max_bytes)` returns `(data = BitArray, address = String, port = Int)` with the numeric sender address and port. `local_address(socket)` and `local_port(socket)` return the bound numeric address and port; `close_datagram(socket)` closes it. Bind accepts IPv4, IPv6 and host names; port 0 asks the OS to choose a port. Send accepts a numeric address or host name and port 1..65535; addresses are resolved and tried in system order as for TCP. Receive requires a positive bound, capped at 65536 bytes; it consumes exactly one datagram and discards any excess bytes. An empty datagram is data, not EOF. Sends require whole bytes and are atomic; expected host failures raise `NetError` values through `Fail`. UDP provides no delivery or ordering guarantee.

Datagrams follow TCP's resource identity, owner, creation guard, teardown and lease rules. One send and one receive may overlap; a second operation in the same direction gives `busy`. Close is idempotent and raises no error, and wakes a blocked receive with `closed` unless its answer already committed. Group cancellation cancels an owned blocked receive and releases its lease, leaving the socket usable until close or entry teardown. The browser and WASI platforms have no `Net`, so a program that needs `i/net.Net` there is a compile error naming it (§10.2).


### 9.8 Keys


`Key(A)` and `Keyed` are built-in types. They are not labelled types: their values are made only by `key.new` and `key.wrap`. A key is the same only as itself (§11.4). Two `Keyed` values are equal when their keys are the same and their values are equal. `Key(A)` does not change (§4.5 rule 7). A `Keyed` holds its value (§11.9).



| Function | Type | What it does |
|---|---|---|
| `new` | `<A>() -> Key(A)` | A new key, the same only as itself |
| `wrap` | `<A>(Key(A), A) -> Keyed` | The value, marked with the key |
| `unwrap` | `<A>(Key(A), Keyed) -> Lookup(A)` | `Found(v)` when `x` was made with `k` from `v`, and `Missing` when not |



`Found(A) = (A)`, `Missing = ()` and `Lookup(A) = Found(A) | Missing` are types of `i/key`, not built-in names.


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


Export types Scheduler, Runner, Group(E), Task(A,E), Channel(A), Event(A,E,N),
Shared(A), Ref(A), and ScheduleError; export only ScheduleError's constructors. The
other constructors and scheduler protocol are private ordinary labelled types,
not opaque types. Group, Task, Channel, Shared, and Ref contain invariant keys;
their parameters are therefore invariant. Event's answer/error/need
parameters are covariant. In `Event(A, E, N)`, `E` is the type of the errors that syncing raises through `Fail` and `N` is a row variable for the other services it needs; `Never` in that place means no service: its private representation keeps typed decoding
functions, rather than Key(A) in an output field. A public Event can hold user
mapping functions and the local handles of its leaves. Such contents affect
portability by the normal walk. The library creates no event that captures its
Scheduler or mutable loop state. A join event keeps its task's scope handle.



Each type below writes its type parameters before its parameter part. `N` is the row of the services that a body needs besides those that the function provides to it.

| Binding | Type |
|---|---|
| `top` | `(random.Generator) <- (Runner) -> Runner` |
| `run` | `<A, N>(() <- (..N, Scheduler, Runner, request.Await, request.Owner, request.Root) -> A) <- (..N, Runner) -> A` |
| `test` | `<A, E, N>(random.Generator, List(List(Int)), () <- (..N, Fail(E), Scheduler, Runner, request.Await, request.Owner, request.Root) -> A) <- (..N, Fail(E \| ScheduleError), Runner) -> A` |
| `scope` | `<A, E, N>((Group(E)) <- (..N, Fail(E)) -> A) <- (..N, Fail(E), Scheduler) -> A` |
| `local_scope` | The type of `scope` |
| `go` | `<A, E, N>(Group(E), () <- (..N, Fail(E), Scheduler, Runner, request.Await) -> A) <- (..N, Scheduler) -> Task(A, E)` |
| `join` | `<A, E>(Task(A, E)) <- (Fail(E), Scheduler) -> A` |
| `yield` | `() <- (Scheduler) -> Nil` |
| `path` | `() <- (Scheduler) -> List(Int)` |
| `random` | `() <- (Scheduler) -> random.Generator` |
| `keep` | `<S, A, E, N>((S) <- (..N, Fail(E), Scheduler, Runner, request.Await) -> A) <- (..N, Scheduler) -> (S) <- (Fail(E), Scheduler) -> A` |
| `channel` | `<A>() <- (Scheduler) -> Channel(A)` |
| `local_channel` | `<A>() <- (Scheduler) -> Channel(A)` |
| `send` | `<A>(Channel(A), A) <- (Scheduler) -> Nil` |
| `receive` | `<A>(Channel(A)) <- (Scheduler) -> A` |
| `send_event` | `<A>(Channel(A), A) -> Event(Nil, Never, Never)` |
| `receive_event` | `<A>(Channel(A)) -> Event(A, Never, Never)` |
| `join_event` | `<A, E>(Task(A, E)) -> Event(A, E, Never)` |
| `always` | `<A>(A) -> Event(A, Never, Never)` |
| `never` | `() -> Event(Never, Never, Never)` |
| `choose` | `<A, E, N>(List(Event(A, E, N))) -> Event(A, E, N)` |
| `map` | `<A, B, E, F, N, M>(Event(A, E, N), (A) <- (..M, Fail(F)) -> B) -> Event(B, E \| F, N \| M)` |
| `sync` | `<A, E, N>(Event(A, E, N)) <- (..N, Fail(E), Scheduler) -> A` |
| `share` | `<A, E>(Group(E), A) <- (Scheduler) -> Shared(A)` |
| `get` | `<A>(Shared(A)) <- (Scheduler) -> A` |
| `ref` | `<A>(A) <- (Scheduler) -> Ref(A)` |
| `update` | `<A, B>(Ref(A), (A) -> (A, B)) <- (Scheduler) -> B` |
| `both` | `<A, B, E, N>(() <- (..N, Fail(E), Scheduler, Runner, request.Await) -> A, () <- (..N, Fail(E), Scheduler, Runner, request.Await) -> B) <- (..N, Fail(E), Scheduler) -> (A, B)` |
| `race` | `<A, E, N>(() <- (..N, Fail(E), Scheduler, Runner, request.Await) -> A, () <- (..N, Fail(E), Scheduler, Runner, request.Await) -> A) <- (..N, Fail(E), Scheduler) -> A` |
| `map_concurrently` | `<A, B, E, N>(List(A), (A) <- (..N, Fail(E), Scheduler, Runner, request.Await) -> B) <- (..N, Fail(E), Scheduler) -> List(B)` |

In tables, `\|` is only Markdown escaping; the i operator is `|`. The schemes
for combinators use one common E and N; subtyping admits either child's error
and needs. No union of invariant Task(Int,E) and Task(String,E) is required:
`both` keeps two separate handles. No local variants of these three combinators
are exported in v1. Local callers write a local_scope explicitly.


`go`, `keep` and the combinators provide `Scheduler`, `Runner` and `Await` to their bodies themselves, so their callers need only the rest of the bodies' needs and `Scheduler`. `Owner` and `Root` are not provided there. `run` and `test` provide a fresh `Root` and a default `Owner`; `test` still needs the calling `Runner` for its nested path and wait route. Host fields add fixed service types to needs, never a constraint that depends on a type. Keys are invariant and obey the value restriction; `Request` and `Event` add no qualified types.

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
Group/Task handle can survive but does not retain its result/state after exit.
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
join's type remains `<A, E>(Task(A, E)) <- (Fail(E), Scheduler) -> A`. After the run/chain checks, process any applicable scope cancellation
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
`NotRunnable(path = List(Int))`, the one member of `ScheduleError = NotRunnable`, after structured cleanup. When
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
functions. The signatures describe bindings with bodies. `RandomError = InvalidBound | InvalidPath`, with `InvalidBound = (Int)` and `InvalidPath = (List(Int))`.



| Name | Type |
|---|---|

| `from_seed` | `(Int) -> Generator` |
| `word` | `(Generator) -> (Int, Generator)` |
| `int` | `(Generator, Int) <- (Fail(RandomError)) -> (Int, Generator)` |
| `float` | `(Generator) -> (Float, Generator)` |
| `split` | `(Generator) -> (Generator, Generator)` |
| `at_path` | `(Generator, List(Int)) <- (Fail(RandomError)) -> Generator` |

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
Await = (wait = (Ticket) -> Nil)
Root = (key = Key(Nil))
Owner = (key = Key(Nil), root = Root)
Waiting = ()
Ready(A) = (A)
Cancelled = ()
Taken = ()
RequestState(A) = Waiting | Ready(A) | Cancelled | Taken
```


| Function | Type | Rule |
|---|---|---|

| `open_root` | `() <- (Root) -> Root` | Open an independent request run within the current entry context |
| `close_root` | `(Root) -> Nil` | Close every live group of this root and the root itself; idempotent |
| `open_owner` | `() <- (Root) -> Owner` | Register a group at the current root; changing Owner does not change that root |
| `close_owner` | `(Owner) -> Nil` | Cancel its outstanding work and remove the live group; idempotent |
| `count` | `(Owner) -> Int` | Number of outstanding requests in this group |
| `count_root` | `(Root) -> Int` | Number of outstanding requests in all groups of this root |
| `settled_owner` | `(Owner) -> Bool` | True exactly when count(owner) is zero |
| `settled_root` | `(Root) -> Bool` | True exactly when count_root(root) is zero |
| `cancel_all` | `(Owner) -> Nil` | Cancel the group's outstanding work at this step; leave the group open |
| `cancel_all_root` | `(Root) -> Nil` | Cancel outstanding work in all its groups at this step; leave them open |
| `wait_none` | `(Owner) <- (Await) -> Nil` | Await until this group's count is zero |
| `wait_none_root` | `(Root) <- (Await) -> Nil` | Await until this root's count is zero |
| `within` | `<A, N>(() <- (..N, Owner) -> A) <- (..N, Root) -> A` | Fresh Owner; ensure close_owner on exit |
| `ready` | `<A>(A) -> Request(A)` | A fresh completed request; available in pure fakes |
| `ticket` | `<A>(Request(A)) -> Ticket` | The same underlying request identity; no answer access |
| `state` | `<A>(Request(A)) -> RequestState(A)` | Reads do not consume the answer |
| `take` | `<A>(Request(A)) -> A` | Ready becomes Taken atomically; other states stop |
| `settled` | `(Ticket) -> Bool` | True for Ready, Cancelled or Taken |
| `cancel` | `(Ticket) -> Nil` | Idempotent; cancels only Waiting, discards a Ready answer |
| `await` | `<A>(Request(A)) <- (Await) -> A` | Borrow the producer, wait, then take the Ready value |
| `await_owned` | `<A>(Request(A)) <- (Await) -> A` | Own a non-resource-producing operation; ensure ticket cancellation on exit |
| `wait_any` | `(List(Ticket)) -> Nil` | Root wait; return when any listed ticket settles |
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
await_owned = <A>(r: Request(A)) -> A {
  resource.ensure(() { await(r) }, () { cancel(ticket(r)) })
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
| `bind` | Bind a UDP datagram socket to an address and port (0 selects a port). |
| `send_to` | Send one datagram to an address and port. |
| `receive_from` | Receive one datagram and its sender address and port. |
| `local_address` | Read a datagram socket's numeric local address. |
| `close_datagram` | Close a datagram socket. |
| `listen` | Listen for TCP connections. |
| `local_port` | Read a listener or datagram socket’s local port. |
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


This declares a function that the host gives. It is the Wasm import `function` in the import module `module`, encoded as in Appendix B. The full type must be written. The type has one shape, its parameter data has positions only, and its parameters and value must be types that the host can take (§10.3). It has no need, or exactly `<- (Fail(E))` with an `E` that the host can take; the host's error is then raised through the caller's `Fail`. Every call of an external function is done when it is reached, like all calls. An external function should have no side effects that i code may see; work with side effects on the host uses host services (§9.7).


#### 10.1.1 JavaScript value declarations


The form is
`@js("expression") name: T`, where T expands to one opaque type (§4.3). No holes, type variables, unions, functions or constructed
resource types. The annotation is required. It creates no i constructor or
cast between labelled types. `i/js.Value = Opaque` is one ordinary use; packages can
define `Element = Opaque` or other opaque types. JS is specific to this attribute;
it is not a target-neutral foreign language feature.


For example, an opaque type defined by a package needs no library rule:


```i
Element = { Opaque }
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

This rule defines acceptance and foreign initializer execution. Run it after ordinary type checking, before optimization, with the same result at every optimization level. There is no per-input specialization, constructor propagation, constant folding, Boolean evaluation, type-directed branch elimination or implementation-chosen widening. There is exactly one branch-pruning rule below.

The finite graph has one node for each ordinary binding's initializer, each source function body (each branch of a chain is one), and each foreign declaration. References are resolved lexical definition identities. Start with all ordinary top-level initializers and emitted entries, exports and tests. Scanning an initializer keeps §2.5: making a function does not scan its body. A reference to an existing binding, including a name bound to another name or a function passed as data, follows its definition and activates all function bodies syntactically contained in that definition. Scanning an activated body visits all expressions and guards, both sides of Boolean operators, and every branch except this section's sole pruning case. A function reached in an activated body is itself activated, even if it is never called. A non-direct initializer scans its nested function bodies too, since it may execute them. Reads of foreign declarations retain them.

Iterate these fixed reference edges to the least fixed point. Every source body has one node shared by all callers; a second call adds no specialized copy. Recursion terminates by marking nodes once. References to bindings of data activate all functions in their definitions, not only a selected key. Higher-order parameters add no inferred callee set: all supplied functions are already activated where they are referenced, stored, returned or passed. This closed program cannot receive an i function from the host (§10.3). So an unknown function parameter cannot hide an unvisited source function. No function-value analysis or optimizer discovery changes this graph.

**The only pruning case.** The platform provides `Target` (§9.7) at every entry, holding the member of `Target`'s data for the target being built. A subject is **certified** only when it is the name bound by `target: Target` in the service pattern of an emitted entry, or a lexical name made solely by `alias = certified_name` (a group does not change this). The entry binding must have **no source value reference anywhere in the compiled program**, including as a higher-order value or recursive call; its imports, exports and entry configuration are declarations, not references. So no i call can provide another `Target` to that body. A parameter, a function result, a key read, a constructor expression, a service pattern of any other function, and a `Target` provided by i code are unknown. There is no propagation through arguments or results.

A function's direct lexical capture of a certified name or alias remains certified. A newly bound or covering name is a different definition and gains certification only through the alias rule. A provide of another `Target` does not change a name already bound; later functions that take `Target` from their caller see the provided value and are unknown. `keep` (§9.4) activates its argument by ordinary reference reachability and grants no certification to the services it keeps. These rules use lexical identity, not run-time analysis.

For the source form `certified_name |> chain` (allowing a one-expression `{ }` group around the subject), omit a branch's guard and body only if its pattern is `Target(C(…))` or `Target(C)` for a resolved constructor `C` that is not the build's member, allowing patterns for the data of `C` and conjunctions with names. For alternatives, this must hold for **every** alternative; any other alternative prevents pruning. Compare resolved constructor identities, not names or target labels. `js` and `nodejs` have different members. A catch-all, a type test or any other pattern is retained. A guarded branch with the build's member is always possibly executed, even if its guard is literally `False`; later branches are retained too. No first-match or coverage reasoning removes any other branch. Calling a named chain, or passing the subject to a function, gives no pruning. Type-check all branches first. Suppressing an impossible branch's references does not remove the pattern-failure `Assert` behavior of a one-branch match, or any other source meaning. This rule changes no inferred need. The compiler keeps this source match until the analysis is done; matches introduced by an optimizer cannot gain certification.

Every retained foreign declaration is initialized exactly once in §10.1.1 order, even if an optimizer removes its reads. An unretained declaration is never initialized. On an unavailable target a retained declaration is a compile error at its declaration and a requiring reference, never null or a late trap. Ordinary top-level `forced = foreign` always retains it. Two retained `@js("({})")` declarations produce distinct objects; reads of one keep its identity within its opaque type (§4.1, §11.4).

Passing `Target` to a helper, taking it in a helper's service pattern, or making the matching constructor also gives no pruning. An uncalled function referenced in retained code still retains its foreign reads. This loss is deliberate and predictable. A direct entry match remains expressible under the certified-entry rule.


The generated artifact and loader metadata are specified in §B.6.1. Host exports always use the fixed ABI conversions, never a dynamically replaced Js converter.


### 10.2 Entry points

These functions are **entry points**:

- **The program entry**: the selected top-level function of the compiled package (§2.6). The default is `main` in `src/main.i`; the name `main` in any other module is ordinary. With `i run NAME` or `i build NAME`, use that `[entries]` target instead. Its parameter group is `()`, and its result is unused. A missing default means no program entry.
- **Tests**: the entries of the collection named by `[test] collection` in `i.toml` (§14.5). `std/test` provides ordinary imported item macros `test`, `skip`, `only`, `fails`, `suite`, and `each`, which contribute private functions and display metadata to `std/test.tests`. For example, `(testlib = (test) = std/test)` makes `test("addition", () { assert(1 + 1 == 2) })` available. These names have no special language meaning. Each selected function has the parameter group `()` and gives `Nil` or `Never`. A test **passes** when it gives `Nil`. It **fails** on a stop (§11.7) or an error that reaches the platform's `Fail`. `fails` reverses that expectation; `skip` reports an entry without running it, and `only` selects focused entries. `suite` prefixes names and provides the services written in it to its test bodies; `each` contributes one test for each written case. The runner compiles the collection once and runs each selected test in a fresh started instance (§B.1). With no configured collection, no tests run.
- **Host exports**: the functions named in `[wasm].exports` (§2.6) of the package that is compiled. The `[wasm]` of other packages is not used. They must be in the exports of their module. A host export has one shape, its parameter data has positions only, and its parameters and value must be types that the host can take (§10.3).

**Platform services.** At an entry the platform of the target provides its services (§9.7): the host services of its table, `Assert`, `Scope`, `Root`, `Owner`, `Await`, `Runner`, `Scheduler`, `Target`, and a top-level `Fail`. For the program entry and tests, that `Fail` takes any error, and an error that reaches it stops the program or fails the test. For a host export, it takes only an error type that the host can take, and the error is given to the host (Appendix B).

The need of an entry, after what the entry's own code provides, may contain only services that the platform of the target provides. Otherwise it is the compile error "`T` is not provided by the target `t`", also when the program catches every error of the operations that need `T`. Checking with no target checks the entry's shape and types, but not its need.

On targets using the asynchronous entry protocol, an entry is **async** exactly when its need has a host service whose declaration is `"async"`, or `Runner`. `Await`, `Owner` and `Root` alone do not make it async. A requested synchronous export that is async is a compile error (§2.6.2). The host waits through Appendix B's protocol. WASI instead keeps blocking imports, synchronous metadata and no async hand-off (§B.7). When the need of an entry has `Scheduler`, `Runner`, `Await`, `Owner` or `Root`, the entry runs in a fresh task run (§9.9) that provides them.

The entry context is opened before the entry runs, and closed on every exit, after structured cleanup and before the result or error is delivered. A stop runs no i finals. A synchronous context cannot hand off: a low-level wait needing hand-off or poll stops with `request: no active entry`. An async entry may return status 0 on its first call; scheduler switches alone do not yield to the host.



### 10.3 Types the host can take

`Nil`, `Bool`, `Int`, `Float`, `String`, `BitArray`, and opaque types (§4.3). Values of opaque types go to the host as opaque references. No other type may go to the host: data, labelled types with data, and functions do not cross. An error type that the host can take is a union of `Nil`, `Bool`, `Int`, `Float`, `String`, and `BitArray`, with no two the same.

Where i code gives a value to the host, an `Int` or `String` member counts as `Int` or `String`, whatever its value set (§4.2): a literal type, a union of literal types of one type constructor (also through a name), or all the values but a finite set, such as `Int - 0`. Where the host gives the value (the value of an external function, the value and errors of a host-service function, and the parameters of a host export), an `Int` or `String` member must have all its values, because the host's values are checked only for their form (§B.2).

```i
@external("app", "mode")
mode: () -> "on" | "off"            // compile error: the host gives the value
@external("app", "log")
log: ("info" | "warn") -> Nil       // correct: i code gives the value
```

The outer `Request` and optional `Result` in a request-mode host declaration describe how work is obtained; they are not additional boundary types. Requests, tickets, tasks, groups, channels and events cannot cross the interface. IO types over Int and constructed `Thrown` and `Mismatch` values are not boundary types; their libraries pass Int or an opaque outcome and decode in i. JavaScript value declarations do not change the value and error lists above. No library-specific exception is added.


## 11. Run-time behavior

### 11.1 Values


No value may be changed. The values are integers, floats, strings, codepoints, bit arrays, data, lists, values of labelled types, functions, boundaries, continuations, keys, keyed values, requests, tickets, and host references.


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

- Data is the same when it has the same shape and its parts under each key are the same. Lists are the same when their parts are the same, in order. Values of labelled types are the same when they have the same constructor and their data is the same. Values with different type constructors, and data of different shapes, are never the same; data and a labelled value are never the same.


- A function, a boundary, a continuation, and a host reference have no parts that may be looked at, so each is the same only as itself: the value made by one evaluation of a function expression or a chain, by one call of `control.boundary` or `control.suspend`, or one reference from the host (§B.4). A top-level function is worked out once (§2.5), so all its uses are the same value. In the same way, a constructor, a built-in function, a library function, and each `@external` declaration are each one value; two declarations of one import are two values. So two function expressions that are written the same way give values that are not the same.



- Each call of `keep` (§9.4) makes a new function, the same only as itself. A key (§9.8) is the same only as itself. Two keyed values are equal when their keys are the same and their values are equal.

- A request or ticket compares by its underlying request identity, which does not change on answer, take or cancel. Ticket identity is not a fresh value on each call of ticket. Request and Ticket are different type constructors and are not equal to one another.
- A value stays the same value when the program copies it from one worker to another (§11.9).

### 11.5 Comparing

`< <= > >=` compare Ints by number. `<. <=. >. >=.` compare Floats by IEEE 754.

### 11.6 Tail calls


A call in tail position uses no more stack. Tail positions are: the last expression of the body of a function expression (§6.8); the body of a branch of a chain that is called; the body of a branch of a match in tail position; the last expression of a block in tail position; the right side of `&&` or `||` in tail position; and the left operand of a provide in tail position. A provide does not change which calls are in tail position.



So in `loop = (0) { Done } | (n) { loop(n - 1) }`, the call `loop(n - 1)` uses no more stack. A match adds no call frame (§6.12).


### 11.7 Stops

The program stops in these cases:


| Case | Reason |
|---|---|
| `panic(m)` is called | `panic: ` and `m` |
| The `Assert` of an entry point gets a failure (§9.6) | `assertion failed: ` and the message |
| An error comes to the program entry or a test (§9.3) | `error: ` and the text form of the error |
| `control.suspend` or `control.escape` with a boundary that is not active, also through a `Fail` used after its `catch` (§9.3, §9.4) | `control: boundary is not active` |
| A continuation is used a second time (§9.4) | `control: continuation was already used` |
| A float operation has a result that is not finite (§11.3) | `float: result is not finite` |
| Making a bit array fails (§8.3) | `bit array: value is too short`, `bit array: length is negative`, `bit array: bytes are not whole`, or `bit array: float is not finite` |
| A value from the host is not correct (Appendix B) | `host: value is not correct` |
| There is no more memory or stack | none |



A JS bridge call that does not expose exceptions uses `panic("js: call threw")` for a thrown exception. Malformed raw outcomes, argument builders or typed host replies use `host: value is not correct`. These stops are not Mismatch errors; explicit checked conversions may return Mismatch. Memory and stack exhaustion remain stops.


Before the program stops, the host gets a **stop message**, in one call of `panic` (Appendix B): the reason, then one line for each source position, with a line feed between lines. The first position is that of the innermost expression in the program source that was being worked out: a call, an operator, a `<<`, or a binding. Then come the positions of the calls in the program source that were running, the innermost first. Calls that were replaced by tail calls are gone, but the first position is always given, also when that expression is a call in tail position: in `f = (n) { assert(n > 0) }`, the stop shows the position of `assert(n > 0)`. A call of a library function, such as `fail.catch` or `resource.ensure`, counts as a tail call when it is in tail position. For an error at an entry point, there is only the position of the entry point's binding.

A **source position** is written `module path:line:column`, for example `app/users:12:5`, with the module path as its own package names it. For a module of a package with a location, the location, `@`, the major version, and a space come first: `github.com/alice/json@1 json/decode:12:5`. Lines and columns count from 1; a column counts Unicode scalar values. The position of an expression is the position of its first token.


A function with no need may still stop for these reasons, and it may never end.


### 11.8 Text form of values

The **text form** of a value is given by `core.inspect` (§13):


| Value | Text form |
|---|---|
| Int | Decimal digits, with `-` before a negative number: `-42` |
| Float | See below |
| String | `"`, then the characters, then `"`. `"` and `\` are written as `\"` and `\\`. Line feed, carriage return, tab, and form feed are `\n`, `\r`, `\t`, `\f`. Other characters from U+0000 to U+001F, U+007F, and U+0080 to U+009F are `\u{XXXX}`, with four upper-case hex digits. All other characters are written as they are. |
| List | `[`, the text forms of the parts with `, ` between them, `]` |
| Data | `(`, the text forms of the positional parts in order, then `label = ` and the text form of each labelled part, in the UTF-8 byte order of the labels, with `, ` between them, `)`. `()` when empty. |
| Value of a labelled type | The constructor name; when its data is not `()`, then the text form of its data: `True`, `Ok(1)`, `User(age = 36, name = "ada")`. |
| BitArray | `<<`, the whole bytes as decimal numbers with `, ` between them, `>>`. If the number of bits is not a multiple of 8, the last k bits come last, written as `value:uk`: `<<82, 2:u2>>` |
| UtfCodepoint | `//utfcodepoint(`, the character, `)` |
| Function | `//fn(`, one name for each key of its shape with `, ` between them, `) { ... }`. The names are `a`, `b`, … in order: position i, and then the i-th label (counting on after the positions), gets the letter `a` + (i mod 26), with the decimal number i div 26 after it when i is 26 or more; a label is written `label = ` before its name: `//fn(a, port = b) { ... }`. A function with several shapes writes the part of each shape group, in the written order of their first branches, joined by ` \| `: `//fn(a) { ... } \| (a, b) { ... }`. |
| Boundary, continuation | `//boundary`, `//continuation` |
| Key, keyed value | `//key`, `//keyed` |
| Request, ticket | `//request`, `//ticket` |
| Host reference | `//wasm(opaque)` |


A **Float** is written in this way. Let d1 … dn be the shortest list of decimal digits, with dn not 0, such that 0.d1…dn × 10^k gives back the same float when read. When more than one list of that length does this, take the one nearest to the exact value of the float; when two are equally near, take the one whose last digit is even. `-` comes first for a negative float, including -0.0. Zero is `0.0`. Let p = k − n.

- The **long form** is used when p is at least L and at most U, and not when p ≥ 0 and the size of the float is 2^53 or more. L is −4 when n = 1, and −2 − n when not. U is 2 when n = 1 or k ≥ 11, and 1 when not.
- In the long form, the digits are written as a decimal number, with `.` and at least one digit after it: `123.0`, `12.67`, `0.0123`.
- In the short form, d1 is written, then `.`, then d2 … dn, or `0` when n = 1, then `e`, then k − 1 as a decimal number with `-` when it is negative: `3.0e26`, `1.0e-7`, `1.0e3`.

The library module `i/float_text` writes this text in i: `float_text.to_string(x)` is the text form of the Float `x`, the same as `core.inspect(x)`.

The text form of a function, a key or a keyed value never depends on which worker made it or holds it. Task handles keep bookkeeping ids under keyed fields, whose text is `//keyed`; only task.path exposes the semantic task path. Requests and tickets show no ids, state or answers.

### 11.9 Workers

A program may run on one or more **workers** at the same time. Which worker does which part of the work cannot be seen. Tasks that run at the same time take their steps (sends and receives, joins, host calls, failures and stops) one after another, in an order that the program does not fix; this order may differ from run to run and with the number of workers. Every run gives a result that some order of the steps gives, and each step does what it would do if the steps were taken one at a time in that order. The number of workers only changes which order happens. So anything that depends on the order of steps may change, for example which sender a receive pairs with, which event of a `choose` commits, whether a step comes before or after the end of a scope, which tasks fail before a cancel reaches them, which of two stops is first, and the order of output. A program whose results do not depend on the order of its steps gives one result. Running out of memory or stack (§11.7) is not covered by this rule. The host chooses the number of workers.


The **captures** of a function made by a function expression are the local names that its body uses and that are bound outside it, after macro expansion; a name counts when it is written in the body, also in a part that never runs. Top-level names are not captures. A function made by `keep` (§9.4) has the captures of its argument and the services it keeps. Other functions have no captures.



The values that a value **holds** are: the parts of data, of a list, or of the data of a labelled value (§11.4); the values of the captures of a function; and the value in a `Keyed` (§9.8). `==` does not look at what a function holds (§11.4). A value is **portable** when it is not a boundary, a continuation, a host reference, a request, a ticket or a `Syntax` value (§14.4), and every value it holds is portable. A value may hold itself, through a local function that calls itself; only the other values it holds then count.


A value that is not portable never goes to another worker. Values may be copied between workers; a copy is the same value (§11.4).

## 12. Built-in names

These names may be used in every module. Other names may cover them.


| Name | What it is | See |
|---|---|---|
| `Int` `Float` `String` `List` `BitArray` `UtfCodepoint` `Never` | Types | §4.2 |
| `Bool` `Result` | Names | §4.2 |
| `True` `False` `Nil` `Ok` `Error` | Labelled types and their constructors | §4.2 |
| `Assert` `AssertionFailure` `Fail` | Labelled types and their constructors | §9.3, §9.6 |
| `Boundary` `Continuation` | Built-in types made by `i/control` | §9.4 |
| `Key` `Keyed` | Built-in types made by `i/key` | §9.8 |
| `Request` `Ticket` | Built-in types operated on by `i/request` | §9.11 |
| `panic` | `(String) -> Never` | §11.7 |
| `assert` | `(Bool) <- (Assert) -> Nil` | §9.6 |



These are normal functions and types. The modules `i/fail` (§9.3), `i/control` (§9.4), `i/resource` (§9.5), `i/assertion` (§9.6), `i/io` (§9.7), `i/key` (§9.8), `i/task` (§9.9), `i/random` (§9.10), `i/request` (§9.11), `i/core` (§13), and `i/syntax` (§14) give the other names. The IO module list is extended with its service definitions.


## 13. The module `i/core`


The module exports the built-in types `Int`, `Float`, `String`, `List`, `BitArray`, `UtfCodepoint`, `Never`, `Boundary`, `Continuation`, `Key`, `Keyed`, `Request`, and `Ticket`. These are the same types as the unqualified built-in names.



These are normal functions: they are called, given, and compared like any other function. Only their bodies are not written in i. Most of them do things that the rest of the language cannot do, such as looking at how any value is made (`inspect`, `hash`). Some, such as `index_of` and `bit_array_concat`, could be written in i, but the runtime does them much faster. They have no need.



| Function | Type | What it gives |
|---|---|---|
| `portable` | `<A>(A) -> Bool` | `True` when the value is portable (§11.9) |
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
| `byte_slice` | `(String, Int, Int) -> Result(String, Nil)` | The bytes from `start` (counting from 0), `length` of them. `Error(Nil)` when they are not all in the string, or when the start or the end is inside a character. |
| `codepoints` | `(String) -> List(UtfCodepoint)` | The characters in order |
| `from_codepoints` | `(List(UtfCodepoint)) -> String` | The string of these characters |
| `codepoint_to_int` | `(UtfCodepoint) -> Int` | The number of the character |
| `int_to_codepoint` | `(Int) -> Result(UtfCodepoint, Nil)` | The character with this number, or `Error(Nil)` when it is not a Unicode scalar value |
| `to_upper` | `(String) -> String` | The string with each character changed to its simple uppercase mapping in Unicode 18.0.0 (UnicodeData.txt). This is one character for one character, and no language changes it. A character with no simple uppercase mapping, such as `ß`, stays the same. |
| `to_lower` | `(String) -> String` | The string with each character changed to its simple lowercase mapping in Unicode 18.0.0, in the same way |
| `index_of` | `(String, String, Int) -> Result(Int, Nil)` | The byte offset (counting from 0) of the first `find` in `in` that starts at `from` or after it. An empty `find` is at `from`. `Error(Nil)` when there is no such `find`, when `from` is less than 0 or greater than the byte size of `in`, or when `from` is inside a character. |
| `bit_size` | `(BitArray) -> Int` | The number of bits |
| `bit_array_to_string` | `(BitArray) -> Result(String, Nil)` | The string, when the bits are whole bytes of correct UTF-8; `Error(Nil)` when not |
| `bit_array_concat` | `(List(BitArray)) -> BitArray` | The bit arrays one after the other. Their sizes may be any number of bits. |
| `inspect` | `<A>(A) -> String` | The text form of the value (§11.8) |
| `hash` | `<A>(A) -> Int` | A number from 0 to 2^32 − 1. Values that are the same (§11.4) give the same number. Which number is not fixed. |
| `never` | `<A>(Never) -> A` | The match on `Never` (§6.12); it is never called with a value |



## 14. Macros and the Lisp surface

### 14.1 Two surfaces

i has one **core tree** and two ways to write it. The **normal surface** is the syntax of §1 to §8, in files whose names end in `.i`. The **Lisp surface** writes the same core tree with brackets, in files whose names end in `.is`. A file uses one surface, chosen by the end of its name. A module path leaves out the extension; a module with both a `.i` and a `.is` file is a compile error. Imports, exports, dependencies and visibility mean the same in both surfaces, and a module written in one surface is used from the other with no difference.

Two trees are **the same code** when they are equal after every group is removed. A `.is` file can be written as a `.i` file only when the normal surface can write its core tree; a quote and a macro definition have no normal spelling.

A **macro** is an i function that runs in the compiler. It takes code as data and gives code as data. Macros are defined only in `.is` files and are used from both surfaces. A macro has no host services: it cannot read files, use the network or read the time, and it gives the same result for the same input on every machine.

### 14.2 Tokens and datums

The Lisp surface uses the tokens of §1 and two more symbols, `` ` `` and `~`. Documentation comments work as in the normal surface. The rule of lines (§1.7) is not used: every line end is a space. `,` does not separate parts; it is only the head of data, of a data type, of a data pattern, and of the imports and exports. Tokens are cut by the longest match (§1.5), so a space is needed where two tokens would be read as one, as between an integer and a `..` after it. After `.` in `(. e k)`, `k` is a `decimal`.

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
Symbol  = (* a symbol of §1.5 that is not a bracket *) .
```

- A `.is` file is a list of datums. The **head** of a `Round` is its first part.
- `~` followed by `..` is always a `Splice`.
- An `Unquote` or a `Splice` may only be inside a `Quote`. A `Quote` may be inside a `Quote` only inside an `Unquote` or a `Splice` of the outer quote.
- A datum is read by its **position**: import item, export item, item, attribute, type, data type item, function result, expression, data item, statement, pattern, data pattern item, bit array segment, or format. Each position admits the rows of §14.3 given for it. A datum that fits no row of its position is a compile error.

### 14.3 How each form is written

In these tables `e` is an expression, `p` a pattern, `T` a type, `R` a result type, `N` a named type, `x` and `a` names, `C` and `U` `UpName`s, `F` a function, and `…` zero or more of the thing before it.

**Files and items**

| Form | Normal surface | Lisp surface |
|---|---|---|
| Imports | `(a/b/c, x = a/b, (C, y) = a/b)` | `(, (/ a b c) (= x (/ a b)) (= (, C y) (/ a b)))` |
| Exports | `(x, U, U(..))` | `(, x U (U ..))` |
| Attribute | `@deprecated("old") Item`, `@m(e…) Item` | `(@ (deprecated "old") Item)`, `(@ (m e…) Item)` |
| Labelled type | `U = (T1, x = T2)`, `U(A) = (A)`, `U = ()` | `(= U (, T1 (= x T2)))`, `(= (U A) (, A))`, `(= U (,))` |
| Opaque type | `U = Opaque`, `U(A) = Opaque` | `(= U Opaque)`, `(= (U A) Opaque)` |
| Name | `U = T`, `U(A) = T`, `U = { (T1, T2) }` | `(= U T)`, `(= (U A) T)`, `(= U { (, T1 T2) })` |
| Binding | `p = e`, `x: T = e` | `(= p e)`, `(= (: x T) e)` |
| Declaration | `x: T` | `(: x T)` |
| Macro call as an item | `m(e…)`, `a.m(e…)` | `(m e…)`, `((. a m) e…)` |
| Items from an expansion | none | `{ Item… }` |

The `{ }` around the right side of a name whose right side is a data type is part of the item: it is kept in `Syntax`, so a macro can tell a name from a labelled type.

**Types**

| Form | Normal surface | Lisp surface |
|---|---|---|
| Union, intersection, difference | `A \| B`, `A & B`, `A - B` | `(\| A B)`, `(& A B)`, `(- A B)` |
| Data type | `(T1, x = T2)`, `(T)`, `()` | `(, T1 (= x T2))`, `(, T)`, `(,)` |
| Function type | `(T1, x = T2) -> R`, `() -> R` | `(-> (, T1 (= x T2)) R)`, `(-> (,) R)` |
| Function type with a need | `(T) <- (Db, Log) -> R`, `(T) <- N -> R`, `(T) <- (..N, Db) -> R` | `(-> (, T) (<- R (, Db Log)))`, `(-> (, T) (<- R N))`, `(-> (, T) (<- R (, .. N Db)))` |
| Polymorphic function type | `<A>(A) -> A`, `<A, N>(A) <- N -> A` | `(-> [A] (, A) A)`, `(-> [A N] (, A) (<- A N))` |
| Function type with several shapes | `{ (T) \| (T, T) } -> R` | `(-> (\| (, T) (, T T)) R)` |
| Named type, value type | `U`, `a.U`, `U(T)`, `x`, `a.x` | `U`, `(. a U)`, `(U T)`, `x`, `(. a x)` |
| Literal type, hole, group | `"a"`, `1`, `-1`, `_`, `{ T }` | `"a"`, `1`, `(- 1)`, `_`, `{ T }` |
| Macro call | `m(T, 3)` | `(m T 3)` |

**Expressions and statements**

| Form | Normal surface | Lisp surface |
|---|---|---|
| Name, member | `x`, `C`, `a.x` | `x`, `C`, `(. a x)` |
| Call | `f(e1, x = e2)`, `f()`, `f(..e)` | `(f e1 (= x e2))`, `(f)`, `(f .. e)` |
| Operators | `e1 + e2`, `-e`, `e \|> f` | `(+ e1 e2)`, `(- e)`, `(\|> e f)` |
| Key | `e.x`, `e.0` | `(. e x)`, `(. e 0)` |
| Data | `(e1, x = e2)`, `(e)`, `()`, `(..e, x = v)` | `(, e1 (= x e2))`, `(, e)`, `(,)`, `(, .. e (= x v))` |
| List, bit array | `[e1, ..e2]`, `<<e1, e2: f>>` | `[e1 .. e2]`, `<<e1 (: e2 f)>>` |
| Block | `{ s1  s2  e }` | `{ s1 s2 e }` |
| Function | `(p1, x = p2) { e }`, `(p) -> R { e }`, `() { e }` | `(-> (, p1 (= x p2)) _ e)`, `(-> (, p) R e)`, `(-> (,) _ e)` |
| Function with a service pattern | `(id: String) <- (db: Db) -> User { e }`, `(f) <- (..env: N) { e }` | `(-> (, (: id String)) (<- User (, (: db Db))) e)`, `(-> (, f) (<- _ (, .. (: env N))) e)` |
| Function with type parameters | `<A, N>(x: A) -> A { e }` | `(-> [A N] (, (: x A)) A e)` |
| Function with a guard | `(p1, p2 ? g) { e }` | `(-> (? (, p1 p2) g) _ e)` |
| Chain, match | `F1 \| F2`, `e \|> F1 \| F2` | `(\| F1 F2)`, `(\|> e (\| F1 F2))` |
| Provide | `e <- (s1, ..env)`, `app() <- live()` | `(<- e (, s1 .. env))`, `(<- (app) (live))` |
| Quote | none | `` `d `` |

**Patterns**

| Form | Normal surface | Lisp surface |
|---|---|---|
| Type test or annotation | `x: T` | `(: x T)` |
| Literal | `1`, `-1`, `"a"` | `1`, `(- 1)`, `"a"` |
| String prefix, suffix | `"a" <> x`, `x <> "a"` | `(<> "a" x)`, `(<> x "a")` |
| Constructor | `C`, `C(p1, x = p2)`, `C(p, ..)` | `C`, `(C p1 (= x p2))`, `(C p ..)` |
| Data pattern | `(p1, x = p2)`, `(p, ..r)`, `()` | `(, p1 (= x p2))`, `(, p .. r)`, `(,)` |
| List | `[p, ..x]` | `[p .. x]` |
| Conjunction | `p1 = p2 = p3` | `(= p1 p2 p3)` |
| Alternatives in a parameter item | `p1 \| p2` | `(\| p1 p2)` |
| Quote pattern | none | `` `d `` |

- In the position of an expression, `(-> …)` is a function: an optional `Square` of type parameters, the parameter data pattern, the result (`_` when no `->` part is written, or `(<- R needs)` with a service pattern), and the body, which is any one expression. In the position of a type, `(-> …)` is a function type. In an expression, `(<- e s)` is a provide.
- `(| …)` in the position of an expression is a chain of two or more functions.
- `(,)` is the empty data wherever data is written. The empty `()` has no position.
- In the position of a data pattern item, `(= n …)` whose first part is a name is a labelled item: `n` is the label and the other parts are its pattern. So `(size = whole = (w, h))` is `(, (= size whole (, w h)))`, and `((a, b) = args)` is `(, (= (, a b) args))`.
- `|`, `&` and `-` take two or more parts in a type. A string interpolation of the normal surface is written as the `<>` of its parts (§1.6).

### 14.4 Code as data

The module `i/syntax` exports these types:

```i
Syntax = Opaque
TypeHandle = Opaque
Node = (TypeView | Name | Discard | Up | Symbol | Int | Float | Text | Round | Square | Curly | Bits | Quote | Unquote | Splice)
TypeView = (TypeHandle)
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
SyntaxError = (message = String, at = Syntax)
```

`Position` is a name for the union of one labelled type with data `()` for each position of §14.2: `ImportItem`, `ExportItem`, `Item`, `Attribute`, `Type`, `DataTypeItem`, `FunctionResult`, `Expression`, `DataItem`, `Statement`, `Pattern`, `DataPatternItem`, `Segment` and `Format`.

- A `Syntax` value is one datum with its span in a source file and its **scope** (§14.6). `Syntax` has no constructors: only the compiler, a quote and `make` give `Syntax` values. `TypeHandle` is a checked type that a typed macro query gives (§14.8); it is valid only in the position of a type.
- The `Syntax` of a piece of code is the datum that the Lisp surface writes for it, also for code from a `.i` file. So the node of the argument `n > 0` is `Node(Round(…))` whose parts have the nodes `Node(Symbol(">"))`, `Node(Name("n"))` and `Node(0)`. `Int` and `Float` atoms are the built-in types in the union of `Node`.
- A macro never sees a group: the compiler removes every group from the code it gives to a macro.
- An atom is one token of its kind. A negative number is the `Round` `(- 1)`. `make` gives a `SyntaxError` for any other atom.
- `==` on two `Syntax` values is `True` only for the same value; `same_code` compares code.

| Function | Type | What it gives |
|---|---|---|
| `node` | `(Syntax) -> Node` | The node of the value |
| `make` | `(Node, Syntax) <- (Fail(SyntaxError)) -> Syntax` | A new value with this node and the span and scope of `like` |
| `same_code` | `(Syntax, Syntax) -> Bool` | `True` when the two are the same datum; spans and scopes are not compared |
| `show` | `(Syntax) -> String` | The datum in the Lisp surface, on one line |
| `show_as` | `(Syntax, Position) <- (Fail(SyntaxError)) -> String` | The code in the normal surface, read for this position |
| `definition` | `(Syntax) <- (Definitions, Fail(SyntaxError)) -> Syntax` | §14.7 |

**Quote.** A quote is written only in a `.is` file; the datum in it is a **template**. As an expression, `` `d `` gives the `Syntax` of `d`, with the scope of the module where it is written. Inside it, `~e` puts in the value of `e : Syntax`, and `~..e` puts in every member of `e : List(Syntax)`; `~..e` may only be a part of a bracket. As a pattern, `` `d `` matches a `Syntax` value: an atom matches a node of the same kind with the same text or value, a bracket matches the same bracket with the same number of parts, `~p` matches one part with `p`, and `~..p`, only as the last part of a bracket, matches the rest as a `List(Syntax)`. Spans and scopes are not compared. A quote pattern may fail.

### 14.5 Syntax macros

A **syntax macro** is a top-level binding in a `.is` file with the attribute `@macro` and no declaration of the same name. Its pattern is one name, its value a function expression whose parameter items are names, and the value must have a subtype of `(Syntax, …) <- (Assert, Definitions, Diagnostics, Fail(SyntaxError), Items) -> Syntax`, with one `Syntax` for each parameter. `@macro` in a `.i` file, and an item with `@macro` in the result of an expansion, are compile errors. A macro is exported and imported as a value is. Its name may only be used in the imports, in the exports, and as the macro of a macro call. A macro may not be used in the module that defines it; a call in a template is data, not a use.

```lisp
(, unless)
(@ macro
  (= unless
    (-> (, cond body) _
      `(|> ~cond (| (-> (, True) _ Nil) (-> (, False) _ ~body))))))
```

A **macro call** is written in these ways in both surfaces, where `m` is a name, or `a.m` with a module name `a`, that is a macro there:

| Position | Written | Arguments | The result must be |
|---|---|---|---|
| Expression | `m(args)`, `e \|> m`, `e \|> m(args)` | Expressions, by position or label | An expression |
| Item | `m(args)`, `a.m(args)` | Expressions | An item, or a `Curly` of zero or more items |
| Before an item | `@m Item`, `@m(args) Item` | Expressions, then the item as the last argument | See below |
| Type | `m(args)` | Types, integer literals and strings, with no labels | A type |

- The parser does not know which names are macros, so arguments are read by the grammar of the position. Arguments are not worked out: the macro gets their `Syntax`. A labelled argument goes to the parameter item with that label. The number of arguments must be the number of parameters.
- No macro call gives a statement or a pattern.
- Before a type definition, the macro adds its result after the definition, which stays as written. Before a binding or a declaration, the result takes the place of the item. The item goes to the last parameter, with the attributes written after the macro. A built-in attribute may not be written before a macro. With several macros before an item, the first is expanded first.
- A result may not contain imports, a quote, an unquote or a splice, or a `Curly` of items anywhere but at the top. `@external`, `@js` and `@host` may be in a result only when the whole attribute datum, with its arguments and its item, came unchanged from the caller's source through the arguments of each run, and at most once.

**Expanding.** One call of a macro is one **run**. Expansion goes from the outside in, and the result is expanded again. A module is expanded in two steps: first the item macros, the macros before items and the type macros inside type definitions, until none is left; then the expression macros and the other type macros. After the first step every top-level name and type definition of the module is known, and the exports are checked. A module is expanded and checked after every module it imports, so a macro's module is done before the modules that use it.

**Limits.** Each run has a fresh budget of 100,000,000 work units, 2^28 allocation words of eight bytes and 10,000 frame units, and expansions may be nested at most 256 deep. These are counted in units of the language, not of the machine; a package cannot change them. Going over a limit is a compile error at the call that names the macro and the limit. A `SyntaxError` from a macro is a compile error at the span of its `at`; a stop in a macro, also by a failure reported to `Assert`, is a compile error at the call.

**Collected bindings.** During an item run the service `Items` of `i/syntax` is provided:

```i
Items = (
  add = (Syntax, Syntax, List((String, String)), Syntax) <- (Fail(SyntaxError)) -> Nil,
  expose = (Syntax) <- (Fail(SyntaxError)) -> Nil,
)
Diagnostics = (note = (String) -> Nil)
```

`add(collection, target, data, at)` records that the function `target`, a binding of this module whose parameter group is `()`, belongs to the **collection** named by the exported binding `collection`, with string metadata `data` (unique keys) and source position `at`. A pair of collection and target may be added once. `expose` adds a name of the module to its exports. Requests of a run take effect only when the run succeeds. A tool reads a collection after checking and chooses which targets become entries (§10.2); importing a module runs none of them. `Diagnostics.note` adds a compiler note, given once for a successful run.

### 14.6 Names in expanded code

Every `Syntax` value has a scope: a module and a set of **marks**. Code read from a file of module `M`, and a template written in `M`, have the scope `M` with no marks. `make(node, like)` gives the scope of `like`. For each run the compiler makes a new mark; it adds the mark to the arguments and to every answer of `Definitions` or `Types`, runs the macro, and then flips the mark on every part of the result. So code from the arguments is as it was, and code the macro made has the mark.

A name is looked up in this way:

1. It is bound by the nearest binding around it with the same text and the same scope.
2. Else, when it has no marks, it is a top-level, imported or built-in name of the module of its scope by the usual rules. A made name of another module reaches only exported names.
3. Else, when it has marks, it is a top-level name that an expansion defined with the same text and scope in the module of the call; or else a top-level, imported or module name of the module of its scope, and then a built-in name. A made name reaches only exported names.

Labels are compared by text only. Every rule that compares names (§2.2, §2.3, §4.3, §4.7, §6.11, §7.1) compares text and scope. A name defined by an expansion with marks may be used only by code of the same run; it cannot be exported, and it may not have the text of a top-level, imported or module name of the module of its scope. A name with no marks whose scope is another module may not be defined. An atom that came from an answer of `Definitions` or `Types` may not be a binder. Services and errors are found by type, not by name, so code from a template and code from the caller share services and error handlers as they would through an ordinary function.

### 14.7 Asking for definitions

```i
Definitions = (definition = (Syntax) <- (Fail(SyntaxError)) -> Syntax)
```

`definition(of)` gives the written type definition named by `of`, an `UpName` or `(. a U)`, with its attributes and parameters, as the `Syntax` of the item. A name bound by a type variable, a type of another module that is not exported or whose constructors are not exported, and a built-in type are `SyntaxError`s; `Bool`, `Nil`, `Result`, `Assert` and `AssertionFailure` give the definitions this document gives them. A value-dependent alias gives its written item. A name that is not found is a compile error at `of`. The definition of a binding cannot be asked for.

In the first step of expansion, a definition of the module being expanded is given only when it is **ready**: written in the module with no macro call left in it. A run that asks for a definition that is not ready is put off and done again later as a new run. When every run left has been put off, they are done one at a time in source order, each answered from the module as it is then; at the end of the step each such answer is asked again of the complete module, and a different answer is a compile error naming the name and the two runs. Runs that wait for one another in a circle are compile errors. So the result does not depend on the order in which runs are done.

### 14.8 Typed macros

A **typed macro** is a declaration and a binding with one name in a `.is` file, both with `@macro`. The declaration is the **signature**: a full function type, with no `_`. The binding is the **implementation**: one `Syntax` parameter for each parameter of the signature, checked against `(Syntax, …) <- (Assert, Definitions, Fail(SyntaxError), Types) -> Syntax`. A typed macro is used only in the position of an expression. Its signature is written `(@ macro (: name T))`, as `(@ macro (: to_text (-> [A] (, A) String)))`.

For type checking, a call of a typed macro is a call of a function with the signature's type: the types of the code around it never depend on what the macro gives, and whether a binding is a direct value and how it is generalized are decided before expansion. The calls of a group of bindings (§4.7) are expanded after the group is inferred, against one unchanged snapshot of its types. The result is checked against the instantiated signature without adding bounds to any type variable of the snapshot, and a result that cannot be proved to fit is a compile error at the call. A generic binding is expanded once, not once per instance.

**Expansion.** A binding group is inferred once, with each typed call standing for its signature; when the group has an error, none of its typed calls is expanded. Its types are kept as a snapshot that every typed call of the group sees, and outer calls are expanded before inner ones. A type variable of the caller is **closed** when it is local to the group, not generalized, and reachable from no published scheme, declared type parameter, generalized variable or non-generalized top-level value; every other variable is **fixed**. For the queries only, a closed variable that occurs only in output places of the instantiated signature is given as the union of its lower bounds (`Never` when there are none), one that occurs only in input places and has upper bounds as their intersection, and any other as one opaque variable; one variable gets the same answer in every query of a run. The result is checked apart from the caller: no variable of the snapshot gains a bound, only new variables of the result do, and an obligation on a variable of the snapshot must be proved from the bounds it already has, or the expansion is rejected. An argument that the result keeps whole keeps its type and the definitions its names resolve to; a binder of the result may not capture a free name of an argument. A new value reference in the result must be to the group itself, to a group it depends on, or to an imported module; a new direct read of a value type is a compile error; and the result may not contribute items, exports or collection entries. An argument that the result drops is still diagnosed from the first check, and no warning is given twice.

```i
Yes = ()
No = ()
NotKnown = ()
Truth = Yes | No | NotKnown

Types = (
  type_of = (Syntax) <- (Fail(SyntaxError)) -> Syntax,
  signature = () -> Syntax,
  subtype = (Syntax, Syntax) <- (Fail(SyntaxError)) -> Truth,
  members = (Syntax) <- (Fail(SyntaxError)) -> List(Syntax),
)
```

| Function | What it gives |
|---|---|
| `type_of(s)` | The type of `s`, an argument of this run or an expression or bound name inside one |
| `signature()` | The type of the call as a function type |
| `subtype(a, b)` | `Yes` when `a <: b` is proved, `No` when it is false and neither has a type variable, `NotKnown` otherwise |
| `members(t)` | The members of `t` (§4.4), in the order of their `show` text; a member of `Int` or `String` with a finite value set gives one literal type per value, and `A & Int` or `A - U` with a fixed `A` is one member |

A type is a set of values that may be written in several ways, so a macro asks `subtype` and `members` instead of matching the `Syntax` of a type. A type that cannot be written in the type syntax is given as a `TypeView`, which `subtype` accepts and `show` prints as a token that is not source text. A fixed type variable is given as an `Up` node with a scope of its own that only names that variable; it may be written in the result without a declaration, but not in a type test. A variable that occurs only in the result of the signature has no evidence, so a typed macro cannot choose by the type that its call is expected to have. A typed macro cannot delay an argument that has a need, and it cannot make items.

### 14.9 Running macros

The macros of a module run in a program made of that module and the modules it imports, compiled for the compiler itself, not for a target. Its entry points get only `Definitions`, `Items` in an item run, `Types` in a typed run, `Diagnostics`, and an `Assert` whose `fail` stops the run. An `@external` function outside the library modules, or an executable `@js` binding, in the needs of a macro is a compile error. `hash` gives fixed numbers there. Each run starts from a fresh state; a result may be kept and used again only when everything the run was given is the same, and it then gets the mark of the new run.

A **compile-time binding** has `@macro`, or has a quote or a function of `i/syntax` in its full dependencies (§2.5). A program selected by its entries or host exports may not need one. `i test` provides the functions of `i/syntax`, so a test may call the functions a macro is made of, give its own `Definitions`, and compare code with `same_code`.

An error in expanded code is reported at the span of the code; code from a template is reported at the macro call, with the template as a note. Warnings are given only for code whose span is in the module being compiled.

`i fmt` formats a file in its own surface without expanding macros. `i expand` writes a module after expansion, in the surface that is asked for; a module with quotes can only be written in the Lisp surface, and `i expand --trace` also writes each run with its macro, arguments and result. The language server shows the expansion of a macro call, gives types and completion in the arguments of a typed macro call from its signature, and goes from a name in expanded code to its definition by its span.



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
| An opaque type | `externref` |


The size of the buffer of an `Int` is 4 × count bytes, of a `String` its length, and of a `BitArray` ⌈bits / 8⌉ bytes.


A function `(P1, …, Pn) <- (Fail(E)) -> A`, or `(P1, …, Pn) -> A` with `E` = `Never`, is passed like this. Services are not passed.


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

- When the work of an async function fails in the host (not an error raised through its `Fail`, which is an answer), the host drops every outstanding request and does not use the instance again.

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

- `services` lists every required host-registration function of the host services that the platform provides to the entries, and `externals` every required external function. A host service that i code provides is not a required host import. Entry async flags and status/result exports follow §10.2, before linking and optimization.

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

- The platform's `Io` writes UTF-8 text plus LF to file descriptor 1 through the checked Wasi adapter (§B.7.1). Positive short writes are retried; zero progress or errors give up that write; print gives `Nil`. It attempts the text first, then LF even if writing the text gave up, and frees buffers on every returning path. It is absent from required-import metadata because it is internally provided. Another host service with the namespace `i:io/v1` is an import like any other.

- Because the runtime imports `fd_write` and `proc_exit`, an import (§B.4) with the import module `wasi_snapshot_preview1` and one of these names is a compile error. Such an import is an external function, or a required host-registration field in the entry provider graph (§9.7). Other functions of `wasi_snapshot_preview1` may be imported.
- Every other required host-registration function and external function is an import as in §B.4, so a host that has them can give them. A module that the host makes before the program, such as one given with `wasmtime run --preload`, cannot use the program's `memory` or `ilang_alloc`, so it can only give functions whose parameters, values, and errors are `Nil`, `Bool`, or `Float`. An import with an `Int`, `String`, or `BitArray` needs a host that defines it with the exports of the instance, such as an embedder, or a module linked in statically. An import with a host reference needs such a host too, because only the host makes host references, and the module then imports `reference_id`.
- A function of an async host service is a blocking import, with the results of a function that is not async (§B.2). Nothing is async, and there are no `ilang_resume:…` and `ilang_result:…` exports.
- It exports `_start`, which calls `ilang_start` and then the program entry. The program must have a selected entry; if not, it is a compile error. When `_start` returns, the exit code is 0.
- Tests are not compiled: a test needs a new instance (§10.2), and a command runs one entry.

When the program runs out of memory or stack (§11.7), there is no stop message: the host gives its own error. The host also decides how deep the stack may be, so ordinary recursion that is not in tail position may stop earlier than in another host.


The WASI platform also provides Stdio, Clock, Entropy, Arguments, Environment
and Fs. A need of Net, Tls, Process or WorkingDirectory that the program does
not provide is a compile error naming the service, target and entry. Custom
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
| alloc | `(Int) <- (Fail(String)) -> Int` |
| free | `(Int) <- (Fail(String)) -> Nil` |
| store | `(Int, Int, BitArray) <- (Fail(String)) -> Nil` |
| load | `(Int, Int, Int) <- (Fail(String)) -> BitArray` |
| fd_write | `(Int, Int, Int, Int) <- (Fail(String)) -> Int` |
| proc_exit | `(Int) <- (Fail(String)) -> Nil` |


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
