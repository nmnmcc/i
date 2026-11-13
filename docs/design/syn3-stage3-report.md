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
- A match that does not cover a member of the subject's sum gives E0516
  alone, as before (a former constructor that no pattern takes is held for
  the coverage error when a pattern takes another member of a declared union
  with it; `Checker::uncovered_member`). An earlier state of this lane also
  gave E0501 (`Point` is not a subtype of `Circle | Square`, corpus fuzz-5:
  `True` is not a subtype of `False`); that was a bug against §5.2.8 rule 11
  ("the coverage error in a match is unchanged").
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

## After the conversion (merge of lane/conv, 2026-11-13)

The conversion landed first, so stage 3 now applies to syntax-three text:
the migrator `types` and its old-syntax output were dropped, and
`syn3/lower.rs` lowers each data definition to a one-constructor type and a
sum to the alias of the union of its members.

- `A(..)` of a sum exports the name and each labelled member with its
  constructor (§5.2.1), in the resolver.
- Sums with a member of their own name (§7.3 manual list) were renamed by
  hand: i/task's `Scheduler` and `Runner` services are
  `Scheduler = (state = SchedulerState)` with `AttachedScheduler` (likewise
  Runner); i/process's sum is `DirectoryMode`; std/http's sum is
  `AnyHeaders`; i/syntax's `Node` is the labelled type of one union item, as
  §2.8's text has it, and its users wrap values in `Node(…)`; test fixtures
  with `X = X | Y` name the sum `XSum`.
- The pattern type of a fitted `C(…)` is the smallest declared union of
  labelled types that contains `C` (§2.12, D4, "as today with the
  multi-constructor `Result`"): `first = (r) { Ok(v) = r  v }` is
  `(Result(A, B)) <- (Assert) -> A` again (types/effects.i, with the closed
  case `closed: (Result(Int, String)) -> Int = (Ok(v)) { v }`, E0501).
- A match reports a member of the subject's sum that no pattern takes with
  E0516 alone (§5.2.8 rule 11), also for a pipe into a clause-block
  literal, which is now a match (the Lisp surface's match).
- A discriminant service wraps the union of its targets' values
  (`Bundle = (value = BundleKind)`); the provider plan wraps the selected
  member (`ConstructionStep::wrap`).
- The known types of W0503 messages fold declared aliases (`Bool`).

## Landing review, 2026-11-13 evening

The three questioned expectation changes have been traced to their source:

- `projects/import_exported`: the converted library explicitly exports the
  labelled type `Hidden`, without its constructor, because `Opaque = Hidden`
  exports an alias that names it. Syntax-three §5.2.1 says, "If the type of an
  exported definition, or the expansion of an exported name, uses a type
  defined in this module, that type must be exported too." The import of the
  type therefore succeeds. Qualified construction and the bare constructor
  still fail; `Private` still checks an unexported type import. The separate
  `export_hidden` fixture still rejects an exported alias of a private type.
  Comments now explain this distinction without changing source coordinates
  or any expectation.
- `types/known`: §6.7 says, "`C(..v, label = e)` is an ordinary call whose
  data has a spread." §2.8 gives `Circle` its own nominal type and one
  constructor. The former E0517 text claimed its type had other constructors;
  it no longer applies. `not_known` still rejects the two-member spread
  (§2.3 requires exactly one known member); `wrong_ctor` still rejects the
  incompatible `Square` data (it has `side` instead of `radius`). Both errors
  are retained as E0501. No program changes acceptance here.
- `types/library.out`: `types/display.rs::tree_with`, in its `Ty::Fn` arm,
  deliberately reads labels from the parameter data's shape, rather than
  `FnTy.labels`, which also carries positional binder names. This was
  introduced by `80bc9296cd`, before the stage-3 merge; the numeric runtime
  modules had stale goldens after conversion. §2.4 says, "Function types
  drop the names and the body. The parameter part is a data type" and
  "Labels are part of the type". Thus `(x: Float)` becomes `(Float)`;
  real data labels remain `label = T`. The library golden test now documents
  the cause. Runtime module definitions and expectations are unchanged.

The worktree started clean at `682d1606d`. Wasm and macro package suites
passed. The requested CLI command reproduced the conversion's known
`a_captured_service_row_keeps_callback_dependencies` failure; its fix is
being tracked in `status-convfix.txt`. Remaining landing checks are in
progress; this paragraph does not claim the branch is green.
