# Syntax three, stage 3: report

Stage 3 of `syntax-3.md` §7.2: in today's syntax every type with two or more
constructors became one-constructor types and an alias; a definition with two
or more constructors is now E0238.

## The migrator

`cargo run -p ilang-syntax --example migrate-syntax3 -- types` rewrites in
place every `.i` file, every ```` ```i ```` fence in Markdown and every Rust
string literal with an `UpName {`; `--check types` counts what is left (0).
One module per subcommand in `crates/ilang-syntax/examples/migrate-syntax3/`,
one row per subcommand in `SUBCOMMANDS`.

- `T(A) { C1(A) C2 }` becomes `T(A) = C1(A) | C2`, `C1(A) { C1(A) }`,
  `C2 { C2 }`; each constructor keeps the parameters its fields use.
- Exports: `T(..)` becomes `T, C1(..), C2(..)`; `T` becomes `T, C1, C2`.
- A constructor whose name is already a type name (a built-in type, its own
  type, another type of the module, an imported name) gets the type name
  `<Name>Case`; uses of the constructor do not change. A wrapper of the
  built-in type of its name (`Int(Int)`) is also printed with §7.3's
  suggestion to drop it.
- Sources that contain `migrate-syntax3: keep` are skipped (the E0238
  fixture).

## By hand

- `i/syntax.Node` as in §2.8, with its users (`std/src/test.is`, Rust tests).
- `i/task` `Scheduler` and `Runner`: a service type is one named type (E0225),
  so each became a one-constructor service whose field is the union of its
  detached and attached states (`SchedulerService(state: SchedulerState)`).
- `std/http` `ResponseStream` and `ClientFunctions` became one-constructor
  types because of the solver bug below.

## Solver bug: error unions in record fields

A record field whose function type has an error union of six or more
one-constructor members fails `E <: E`; the member reported depends on the
order of the members. The same check passes for a field of a custom type
and for a function parameter, so the fault is in the records graph, which
stage 6 deletes. Lane s67 should confirm that this passes there:

```text
(i/fail)
(main)

E = A | B | C | D | Q | P
A { A }
B { B(String) }
C { C(String) }
D { D(String) }
P { P(String) }
Q { Q }

f = (n: Int) -> Int ^ E {
	n |> {
		0 -> fail.raise(A)
		_ -> n
	}
}

R = (read: (Int) -> Int ^ E)

g = () -> R {
	(read: (n) { f(n) })
}

main = () { Nil }
```

Today: `error[E0501]: \`Q\` is not a subtype of \`P | Q | { D | { C | { A | B } } }\``.
With five members, or with `R { R(read: (Int) -> Int ^ E) }`, it checks.

## Host ABI

`Bool = True | False` reaches the host as before: a host export
`(Int) -> Bool` has `"result":"bool"` in `ilang.abi`, an error `^ Bool` has
`"errors":["bool"]`, and the JavaScript bindings give `true`/`false`.

## Behaviour changes seen in the tests

- A refutable binding of an unknown value narrows it: `first = (r) { Ok(v) = r  v }`
  is `(r: Ok(A)) -> A` with no `Assert`, not `(r: Result(A, B)) -> A ! Assert`
  (lane js's D4 rule widens to `Q | r - Q` only when another constructor's
  value is known). Fixtures that pass an `Error` give the parameter the type
  `Result(…)` now (`tests/spec/9/assert.i`, `crates/ilang-check/tests/codegen.rs`);
  a recursive top-level value needs a written type on the part
  (`tests/spec/7/may_fail_group_ctor.i`).
- Types are more precise: `named : (r: Result(A, B)) -> Ok(A | 0)`,
  `not_known : (s: Shape) -> Circle`; a constructor's type is its own
  (`Some: (A) -> Some(A)`).
- A one-clause match that does not cover gives E0501 (`Point` is not a
  subtype of `Circle | Square`) besides E0516.
- `Circle(..v, …)` with `v: Shape` gives E0501, not E0517's message.
- `field_shared` no longer joins the constructors of a former family.
- Coverage witnesses show a one-constructor type as its constructor
  (`("a", False)`).

## Decisions after the first report

- A build-discriminant service stays one nominal type; the spec's `Target`
  is `Target { Target(TargetKind) }` with `TargetKind = Js | NodeJS | Wasi |
  Native | Host`. The two `provider_graph.rs` discriminant tests were deleted:
  the provider code wants the discriminant's constructors to be the
  service's own, and the provider plan is being removed (E5).
- Display folds an alias's expansion also inside a larger union, the alias
  with the most members first: `String | True | False` is `String | Bool`,
  `Result(Int, Int) | String`.
- The order dependence of `value_types.rs` came from record graphs, not
  from alias folding: the record of std/http's request body reader showed
  its error union in checking order. That reader is now the custom type
  `BodyReader(E, N, M)`; the record solver bug above is the same family.

## Not solved

- The record-field solver bug above (stage 6 deletes records).
