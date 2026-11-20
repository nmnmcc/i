# Rules the standard states that the compiler does not implement yet

State on 2026-11-21, against `docs/spec.md` at `2c2923f8a`. Each row is a rule
in the standard that the compiler differs from. The list is the tracked delta
between the definition and the implementation: a row is removed by the commit
that implements it, and its ID is the one used in the PR (`fixes B30`). IDs
match [`consistency-audit.md`](consistency-audit.md); that file carries the
decision, owner line and migration count.

**Status** is `open`, `in progress`, `fixed`, or `not reproduced`. **Repro** is
the shortest program or command. Probes lived in `/tmp`; each row can be
re-created from its repro. An `impl` row is re-run after a build of the audit
revision before a fix is merged, because the binary used for this pass is from
an ancestor and `target/` is shared.

Severity: `S1` wrong program, crash, hang, or valid code rejected; `S2` wrong
acceptance or an unusable rule; `S3` misleading diagnostic; `S4` wording.

## The compiler accepts what the standard rejects

| ID | Sev | One line | Rule | Owner | Status | Repro |
|---|---|---|---|---|---|---|
| B1a | S1 | Non-principal typing is chosen silently, with no E0504 | §4.7 principal type | check | open | `both = <A>(x : { A \| List(A) }) -> List(A) { [] }`; `r : List(List(Int)) = both([1])` → E0501, no E0504 |
| B2 | S1 | A provided key that a rigid row variable may carry but does not state is accepted | §4.8 rule 5b, §9.2 | check | open | `<N>(body : () <- N -> Int) -> Int { body() <- (Clock(now = () { 1 })) }` → exit 0 |
| B7 | S3 | `(..)` on a name whose expansion is not a local labelled type is accepted | §2.4 | check | open | `X = Int` and `(X(..))` → exit 0 |
| B8 | S3 | Providing a non-service value is accepted | §9.2 | check | open | `main = () { 1 <- (5) }` → exit 0 |
| B9 | S2 | The promised warning for an unneeded provided key and for a captured `Fail` does not exist | §9.2, §9.3 | check | open | `grab = () <- (f : Fail(String)) { () { f.raise("late") } }` → no warning; run → `control: boundary is not active` |
| B10 | S3 | A bare `<- Int` is accepted where `Int` is not a type parameter | §3 | check | open | `f : (Int) <- Int -> Int = (n : Int) { n }` → exit 0 |
| B11 | S3 | A type may be named `Opaque` | §3, §4.3 | check | open | `Opaque = Int` → exit 0 |
| B12c | S3 | A missing union member or a lone guard gives a subtype error and a coverage error | §7.4 | check | open | match `{A \| B \| C}` with no `C` branch |
| B12d | S4 | A type test in a bit-array segment gets `expected a name, found Int`, never mentioning formats | §8.1 | syntax | open | `<<x : Int>>` |
| B15 | S3 | A line break inside a type-group `{ }` is accepted | §3 | syntax | open | `x : {` NL `Int` NL `} = 1` → exit 0 |
| B20 | S4 | E0301/E0308 omit `@js` and `@macro` from the built-in attribute list | §2.2 | check | open | `@sync main = () { () }` → E0301 lists three attributes |
| B23 | S3 | A built-in type may be shadowed silently (`Int = (a = String)`) | §2.5 | docs | open | `Int = (a = String)` then `x : Int = 1` → confusing E0501 |
| B24 | S4 | Misplaced `///` and `////` are accepted with no diagnostic | §1.2 | syntax | open | `///` on a statement; `////` after an item |
| B25 | S4 | Float-literal underflow is accepted; rounding is not stated | §1.6 | syntax | open | `1.0e-400` → exit 0 |
| B28 | S4 | The "library paths" rule cannot be triggered | §2.1 | docs | open | `src/i/io.i` becomes `pkg/i/io` |

## The compiler rejects what the standard accepts

| ID | Sev | One line | Rule | Owner | Status | Repro |
|---|---|---|---|---|---|---|
| B3 | S1 | Data values and data types reject positions mixed with labels, and any spread after a position | §3, §6.4 | syntax | open | `x : (Int, a = Int) = (1, a = 2)` → 2× E0101; `(0, ..t)` → E0101 |
| B12a | S1 | A `..`/`..r` rest in a plain data pattern is rejected; `..r` after a constructor is rejected | §7.1, §7.2 | syntax | open | `((x, ..))` → E0101; `P(a = x, ..r)` → E0101 (this is P13) |
| B12b | S2 | `User(name, age)` is accepted but `label = pattern` gives the label shorthand | §7.1 | check | open | `U(age, name)` binds by label |
| B30 | S1 | A macro result containing an `@macro` item panics the compiler when the template is past the calling file's end | §14.5, §14.9 | macro | open | macro returning `@macro`; exit 101, `ilang-macro/src/project.rs:454` |
| B31 | S1 | Macro-made code reaches a non-exported top-level name of the macro's module | §14.6 | macro | open | macro in `m.is` returns a name that `m` does not export; caller runs it |
| B32 | S3 | An expanded result of the wrong form is reported at the template only, with no call location or note | §14.9 | macro | open | macro returns a non-expression in expression position |
| B33 | S3 | A top-level binding whose value is a quote fails E0322 | §14.9 | macro | open | `.is` file: ``x = `a`` → E0322, also when unused |

## Silence or wrong result at run time

| ID | Sev | One line | Rule | Owner | Status | Repro |
|---|---|---|---|---|---|---|
| B1 | S1 | With two or more workers, cancellation whose final blocks hangs the run instead of `panic("task: deadlock")` | §9.9 l.1947 | runtime | open | gated child whose final does `sync(never())`; `timeout 300 i run` → rc=124; `I_WORKERS=1` → deadlock panic |
| B4 | S2 | `i check` passes an entry whose need nobody provides; only build/run reports E0523 | §9.2 | check | open | `main` calls a function needing `Clock`; `i check` exit 0, `i run` E0523 |
| B5 | S3 | Negative floats print `-1.5`/`-0.0` instead of `-.1.5`/`-.0.0` | §11.8 | runtime | open | `core.inspect(-.1.5)` → `-1.5` (this is SX16) |
| B6 | S3 | `.0` on a labelled value is E0512 | §6.7 | check | open | `Pair = (Int, Int)`; `Pair(1, 2).0` → E0512 |
| B13 | S2 | An import cycle also prints ~20 unrelated errors inside `stdlib/src/test.is` with a wrong old-syntax hint | §2.1 | package | open | two modules importing each other in a package with `std` |
| B16 | S4 | `{ Int } -> Int` checks, and the form error becomes a later subtype error | §3 | check | open | `h : { Int } -> Int = (x : Int) { 1 }` → E0501 |
| B17 | S4 | Positional duplicate parameters also emit a label E0219 | §6.11 | check | open | `f = (x, x) { x }` → E0218 + E0219 |
| B18 | S4 | W0501 lands on the first branch and blames an "earlier clause" when the subject type kills it | §6.12 | check | open | `(3, 4) \|> ((0, y)) { y } & ((x, _)) { x }` |
| B18a | S4 | A failed type-test pair also emits a false W0501 on the second clause | §4.8 rule 11 | check | open | `v : Box(Int \| String)` with `Box(Int)`/`Box(String)` tests |
| B18b | S4 | E0202 advises `f(A) = …`, which the parser rejects | §4.7 | check | open | `x : Foo = 1` → hint; `f(A) = …` → E0101 |
| B27 | S4 | Interpolation in a literal type is a generic parse error | §1.6 | syntax | open | `x : "\("a")" = "a"` |
| B34 | S4 | `i fmt --to is` swaps a doc comment and the preceding plain comment | §1.7 | fmt | open | format a block with those two comments |
| B36 | S4 | A host-service error also gets a spurious W0501; macro stops show `probe/_compiler_macro_0` | §4.8, §11.7 | check | open | `@external("x", "y") f : () -> Int` used wrongly |

## Rules ahead of the compiler (from earlier reviews)

| ID | Sev | One line | Rule | Owner | Status | Repro |
|---|---|---|---|---|---|---|
| C03 | S2 | A scope captured by user suspension stays in the caller's chain, so `go` outside the captured body succeeds and the run later deadlocks | §9.9 R1, §9.4 | runtime | not reproduced (2026-11-21) | scope returned from its scope, then `go` on it; the three tried shapes now stop with `go outside its scope`/`handle of another run` |
| C04 | S3 | `keep`/`task.keep`/`go` retain the whole environment, so an unused Request-bearing service makes an identity callback nonportable | §9.4, §9.9, §11.9 | runtime | open | `core.portable` over a kept function with an unused Request service |
| EF10 | S3 | Row capture and nonliteral provision are rejected at the parser gate | §6.8, §9.4 | syntax | open | `..env : N` projection, nonliteral provide |
| EF11 | S3 | `i/resource` lacked `Scope`/`scoped` and the two-argument `acquire` | §9.5 | stdlib | fixed in source (2026-11-21), re-verify | `(Scope) = i/resource`; `resource.scoped` at `stdlib/src/resource.i` |
| P15 | S4 | No never-used warning for a catch-all after an `Int` branch, or a two-position branch in an inline one-position match | §7.4 | check | not reproduced (2026-11-21) | `f = (n : Int) { n } \|> …` with a trailing catch-all |
| SX16 | S3 | `core.inspect(-.1.5)` gives `-1.5`, whose pasted expression fails Int negation typing; negative zero needs `-.` | §11.8 | runtime | open | same as B5 |
| T12 | S2 | `Inv(Int) & Inv(String)` reports E0503 and tries to equate `Int`/`String` instead of `Never` | §4.4 | check | open | `Inv(A) = (put = (A) -> A)`; intersect the two |
| T13 | S2 | `(Never)`, `(Never, Int)`, `Box(Never)` and `Loop = (Loop)` are not equated with `Never` | §4.1, §7.4 | check | open | pass each to `core.never` → E0501 |
| T14 | S2 | `List(A) - List(Int)` is accepted and adds constraints; the right side must be holes | §4.4 | check | open | `x : { List(Int) - List(1) } = [1]` → E0501 |
| T15 | S3 | `Phantom(A) = ()` is rejected E0221; it must retain an unused invariant parameter | §4.5 | check | open | `Phantom(A) = ()` |
| T16 | S3 | A well-formedness check is missing: `Bad = Thunk(1)` with `Thunk(N) = (run = () <- N -> ())` is accepted | §4.3 | check | open | the two definitions |
| T23 | S3 | Polymorphic function types inside data are rejected at the syntax gate | §4.6, §4.7 | syntax | open | `T = (f = <A>(A) -> A)` → E0101 |
| POLY1 | S3 | Written polymorphic unions are refused before name resolution | §4.6, §4.7 | syntax | open | `U = { <A>(A) -> A } \| { <B>(B) -> B }`, then `id : U = …` |
| MP08 | S3 | Macro budget is charged on lowered statements/locals, not source nodes | §14.5 | macro | open | `depth(4000)` hits the frame limit |
| DOC3 | S3 | `i build --target js/nodejs` writes no `.mjs`/`.d.ts`, no `ilang.js` section; `@js` is E0301 | §2.6.2, §B.6.1 | host | open | build one-file project for both targets |

## A program that passes the checker and traps

| ID | Sev | One line | Rule | Owner | Status | Repro |
|---|---|---|---|---|---|---|
| T8/F2 | S1 | A function written `<A>(x : A) -> A` whose body gives another type still lends callers the known type `A`; a selection can pick the wrong branch and trap | §4.7, §4.8 | check | open, not reproduced 2026-11-21 | `f = <A>(x : A) -> A { … 1 }` and `n : Int = choose(f("s"))` with a `{Int \| String}` branch |

Two fixes are known for T8/F2: written parameters as rigid (rejects programs
§4.7 accepts today) or no known type from the writable part (rejects nothing).
It needs the owner; see `consistency-audit.md` A8.
