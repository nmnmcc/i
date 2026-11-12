# Syntax three, stages 9 and 10: what the checker hands to the run time

This is the agreement between the checker lane (stage 9 and 11, branch
`lane/s9`) and the run-time lane (stage 10, branch `lane/s10`) of
`docs/design/syntax-3.md` §7.2. It says how a multi-shape function literal, a
direct call of a named chain and a call through a multi-shape value look
after checking. Names of Rust items are those on `lane/s9`.

## 1. Shapes

A shape is a `ilang_check::types::ShapeId` (`crates/ilang-check/src/types/shape.rs`):
the number of positions and the sorted set of labels. `ShapeId::positional(n)`
is the shape of `n` positions. The parts of a shape are in canonical order:
the positions, then the labels in UTF-8 byte order. A function of shape `S`
with `k` keys is an ordinary function of `k` parameters in canonical order
(`$fn{k}` in the back end, as §6.4 says). `(a, b)` and `(a, port = p)` are
both functions of two parameters.

## 2. A chain with several shape groups

The syntax-three front (`crates/ilang-syntax/src/syn3/lower.rs`) partitions
the branches of a chain by shape. A chain is a **multi chain** when it has
more than one group or when its one group has a label.

A group is lowered exactly as a one-shape chain is lowered today: an
`ast::ExprKind::Fn` (one branch, no guard), or a function of fresh
parameters over a `Case` (rule 11). A labelled parameter item `label = p` is
the parameter with the pattern `p` at the canonical place of `label`.

**Bound to one name with no written type** (`area = F1 | F2`, top level or in
a block): the binding becomes, in this order,

```text
area%g0 = <the function of group 0>      // hidden binding, an ordinary function
area%g1 = <the function of group 1>
area    = Multi[(shape 0, area%g0), (shape 1, area%g1)]
```

Groups are numbered in the order of their first branch. A hidden name
`name%g<i>` cannot be written in source. When `area` is exported, the hidden
names are exported with it.

**Anywhere else** (an argument, a list item, a binding with a written type):
`Multi[(shape, function literal), …]` with the group functions in place.

Tree nodes:

```rust
// crates/ilang-syntax/src/ast.rs
ExprKind::Multi(Vec<MultiGroup>)
pub struct MultiGroup { pub positions: u32, pub labels: Vec<Ident> /* sorted */, pub value: Expr }

// crates/ilang-check/src/hir.rs
ExprKind::Multi(Vec<MultiGroup>)
pub struct MultiGroup { pub shape: ShapeId, pub value: Expr }
```

In the HIR, `value` is `ExprKind::Name(ValueRef::Def(_) | ValueRef::Local(_))`
of the hidden binding, or an `ExprKind::Fn`. Its checked type is a function
type of `k` parameters (the keys of `shape`, canonical order).

## 3. A direct call of a named chain: no new fact

A name has a selection table when its binding is `name = Multi[…]` with hidden
group bindings, or `name = other` where `other` has a table (a direct alias,
§2.6.7), in the same module or imported.

The resolver (`crates/ilang-check/src/resolve.rs`) reads the shape of a call
`f(…)` from its syntax, when `f` is a name with a table: the number of
positional arguments and the labels. It then **rewrites the call** to a call
of the hidden binding of that group, with the arguments in canonical order
and no labels:

```text
connect("a.com", port = 8080)   =>   connect%g1("a.com", 8080)
area(2.0)                       =>   area%g0(2.0)
x |> area                       =>   area%g0(x)
```

So after resolving, a direct call of a named chain is an ordinary
`ExprKind::Call` whose callee is `ExprKind::Name` of an ordinary function
binding: the checker, the numeric lowering and the back end need nothing new,
and there is no dispatch (§6.4 "Direct calls"). When the written order of the
arguments is not the canonical order, the call is wrapped in a block that
binds the arguments in written order first, so the order of evaluation is the
written one. No group with that shape is the compile error `E0530`.

Selection by known types (stage 11) only changes the *type* of such a call
(the result is the union over the selected branches of that group). It never
changes which function is called: run time is first match inside the group.

## 4. A multi-shape value

`ExprKind::Multi(groups)` that is still in the HIR after checking is a
multi-shape value. Its type is `{ B_g0 | … } -> R_all` (§2.6.5).

One exception, recorded by the checker: when a `Multi` expression has an
expected type that is one function type of `n` parameters and the `Multi` has
the group `ShapeId::positional(n)`, the value is **that group's function
alone** (selection by an expected type, §2.6.4). The checker records it in

```rust
// crates/ilang-check/src/types/mod.rs, on the result of checking
Types::selected_groups: HashMap<Span /* of the Multi expression */, u32 /* index into groups */>
```

The back end lowers a `Multi` whose span is in `selected_groups` as its
`groups[i].value` and nothing else. Every other `Multi` is built as `$multi`
(§6.4): tag 7, field 2 = −1, and a `$values` array of the pairs
(shape id as `i64`, group closure), in the order of `groups`. The run-time
shape id of a `ShapeId` is §6.4's: `n` for `positional(n)`; for a shape with
labels, an index at or above 2^32 into the static shape table. The back end
owns that numbering; it only has to be the same at `$multi` construction and
at call sites.

Until stage 10 is merged, `lane/s9` lowers a `Multi` that is not in
`selected_groups` as the tuple of its group closures, so that programs build;
such a value cannot be called (§5), and `core.inspect` shows a tuple. Stage 10
replaces that lowering (the place is marked `syn3-stage10` in
`crates/ilang-wasm/src/lower.rs`).

## 5. A call through a value (an indirect call site)

An indirect call is an `ExprKind::Call` whose callee is not a name with a
table. Its static shape is read from its `args`:
`ShapeId::new(number of args with label == None, labels of the others)`;
today's calls have no labels, so the shape is `positional(args.len())`, which
is the `n` the back end already has at each of the 11 sites. The arguments
are passed in canonical order.

In the type checker, a multi-shape type is used where a one-shape function
type is wanted (`{ (Int) | (Int, Int) } -> R <: (Int) -> R`) at one place:
`Solver::constrain` for two function types, when the left one is multi-shape
(`groups::is_multi`) and the right one is not. Stage 9 reports the temporary
error `E0531` there ("a multi-shape function value can only be called
directly by its name until the run time has multi-shape values"). **Stage 10
lifts it by deleting that one `self.fail(…)` line** (marked `syn3-stage10` in
`crates/ilang-check/src/types/solve.rs`); the constraint beside it (argument
data against the member of the call's shape, results, needs) is already
there. After that every closure call site can receive a `$multi` and needs
the slow path of §6.4.

## 6. What stage 10 does not need from the checker

- Which branch inside a group runs: that is the group function's own `Case`.
- Tables in package interfaces: an imported chain is called through the same
  hidden bindings, which are ordinary exported functions.

## 7. Stage 10 (lane/s10): what the run time does

- A run-time shape id is `n` for `n` positions and `set << 32 | n` for a
  shape with labels, where `set` (from 1) is the label set of data in
  `ir::Program::shapes` (`ir::shape_id`). Closure field 2 is that id.
- `Value::Multi(Vec<(ShapeId, Atom)>)` makes a `$multi`; a closure call site
  tests `$clo{n}` once (`br_on_cast_fail`) and otherwise calls
  `$dispatch{n}` with its shape id; the dispatcher ends in
  `return_call_ref`, so tail calls and captures see no extra frame.
  `$call0`, `$call1` and the handler call of `$boundary_core` dispatch too.
- `FailKind::MultiCall` / `E0551` are deleted; two upper bounds of one
  variable that are function types of different shapes are no longer an
  arity failure.
- Not yet: a labelled call through a value (`c("b.com", port = 8080)` with
  `c` a parameter) is still E0506 in the checker; the back end lowers it
  (`Callee::Shaped`) once the checker lets it through.
