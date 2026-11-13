# F1: known types are facts

Assessment, lane `rtknown`, 2026-11-14. This assesses the supervisor's
narrower fix before implementation.

## What a fact means

For an expression evaluated in a reachable source context, a known fact
must contain every value that the expression can produce in that context,
for every caller, provider, callback and subsequent use allowed by its
inferred type. A hole places no restriction. A type variable in a scheme
is universally quantified; matching it must respect the direction in
which values flow. A fact about a function's result is conditional on the
function being called within its domain. A pattern supplies facts only
inside a branch that actually matched. A fact about a mutable cell must
hold across all writes, not just its initial value.

This is a semantic upper bound on possible runtime values. It need not be
an upper bound on the *stored inferred monotype*: after the successful
binding `x = "a" = s`, the inferred type can remain `String`, while the
reachable value of `x` is `"a"`. Conversely, an inference lower bound,
an initial value, or a suggested expected type is not such a fact.

## Existing computation

`crates/ilang-check/src/types/known.rs` converts written types (`kconv`),
constructors and schemes (`ctor_known`, `scheme_known`, `other_known`),
propagates pattern information (`fit_known`, `member_for`), and matches
known arguments into polymorphic results and expected types (`match_k`,
`ksubst`, `call_known`). `hidden_inputs` identifies variables whose values
can arrive other than through an exposed covariant argument position.
`whole_sigma` currently applies `whole_lits` to those matches. That
changes `0` to `Int`, but cannot justify even the constructor `Int`: a
service, callback, invariant cell, or later caller may supply `String`.
`known_bound` builds an inference constraint for a key access.

`groups.rs` retains branch tables and uses `select`, `key_disjoint`,
`select_at_call` and `select_by_expected` to discard branch results from
known argument/expected parameter types. This is an operation that needs
facts: a skipped branch must be impossible at runtime.

`infer.rs` carries expression and binding knowledge through `infer_k`,
`call`, `function`, `local_binding` and top-level group generalization.
The call pass also supplies expected types to deferred callbacks. The
binary-expression pass issues W0503 from disjoint knowledge. `field`,
`tuple_index` and `data_part` use the structural knowledge to choose an
access layout and constrain the actual inferred receiver against a bound.
`pattern.rs` distributes this knowledge to places and catch-alls, applies
rule 12, and combines branch result knowledge.

## Assessment of the narrower fix

Keep two channels. The factual channel replaces a hidden-input match by
`_`, including constructor information, in both call results and expected
callback parameters. Selection (including selection by an expected
function type), rule-12 factual narrowing and W0503 consume only this
channel. The structural channel retains the existing whole-literal hints.
A `.k` or `.f` access can use that channel to find a layout, provided the
receiver's inferred type is constrained to the corresponding bound and
the result's inferred type comes from that bound. Projected hints must
not become projected facts. Carry both channels through aliases, data,
patterns, nested function results, and weak/generalized bindings.

Under these conditions the narrower fix is sound for F1: selection cannot
remove a feasible branch using a hidden-input guess, and a wrong shape
hint used for an access produces an ordinary type error through its bound.
Simply changing `groups.rs`, or widening scalar literals, is insufficient.
Written type parameters must separately become rigid (F2/T8); otherwise
a written polymorphic result itself lies about the function's behavior.

This preserves the useful field inference example:

```i
(i/list)
f = () { list.fold([1], (n = 0), (acc, x) { (n = acc.n + x) }).n }
```

The accumulator hint supplies the record layout, and accessing `acc.n`
constrains all accumulator values to that layout. Replacing all hidden
knowledge with holes, including access hints, would reject it.

## Acceptance limitation for the owner

The request that *every* currently inferred program remain accepted does
not follow from this narrower fix. Some safe programs rely on the same
unjustified branch-result selection as the unsafe probes. For example:

```i
State(A) = (get = () -> A)
cur = <S>(start: S) <- (state: State(S)) -> S { state.get() }
choose = (x: String) { "string" } | (x: { Int | String }) { 1 }
f = () {
    n: Int = choose(cur(0)) <- (State(get = () { 1 }))
    n + 1
}
```

The program runs safely today. Before its provider is checked, however,
`cur(0)` has no factual `Int` bound: the same callee can read `String`.
Conservative selection retains both results and the `Int` annotation may
fail. Similarly a callback which always returns its initial constructor
can be safe while the initial constructor alone is insufficient evidence
for selecting its parameter's branches. Restoring acceptance generally
would require a further rule (for example deferred selection after solving
all hidden inputs), beyond the specified selection-before-inference rule.

The supervisor expressly authorizes fixing the unsafe behavior if full
acceptance preservation fails. Implementation will retain access hints,
add regressions for all reproduced unsafe paths, and record actual newly
rejected repository programs and small compatibility examples here. No
existing regression will be weakened to hide an unexplained difference.
