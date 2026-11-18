# Principality in i: argument and remaining obligations

This is an audit of the language in [the spec](../spec.md), especially
[§4.7](../spec.md#47-type-inference), against checker revision `180e00cf7`.
It is not a proof that the compiler finds a principal type for every binding.
The original observations below are against that audit revision. The two
checker defects in §§1–2 are now fixed: ambiguous intersection specialization
is rejected under §4.7, and deferred empty service meets retain E0515 through
generalization. Their correction commits are recorded in those sections. Owner Decision 1,
applied on 2026-11-15, changes the original method-only callback probe to
an acceptance control: `Read(Never)` is inhabited. The deferred rejection
regression now uses a required value field; see §2 and the merge ledger below.
Other extensions have conditional arguments and unproven parts; §4.7's
principality sentence remains a compiler requirement.

For a fixed source program and environment, a principal scheme must be
admissible and subsume every other admissible typing. Here “subsume” includes
instantiation **and** the subtype rule of [§4.8 rule 1](../spec.md#48-typing-rules).
Instantiation alone is insufficient: an identity can be used at
`(Int) -> Int | String` by subsumption. Scheme variables may be instantiated;
monomorphic variables shared with the environment may not be independently
instantiated. For a weak top-level value, the environment includes the other
uses in the checked program. For selection, it also includes the binding's
selection table and its known-type facts. These distinctions are necessary
to give the requirement a precise meaning.

There are three different questions throughout:

1. Does the specified typing relation have a principal solution?
2. Does this algorithm retain all solutions until generalization or completion?
3. Does the shown type faithfully describe that solution, including sharing and
   obligations that are not visible in its text?

Deterministic output, successful examples, and type safety do not individually
answer the first two questions. The probes below test candidate failures, not
the universal property.

## 1. The algebraic base

**Verdict: the order-dependent inference defect is fixed in `685a1b04c`,
with follow-ups `cde147079` and `c51d1d5a5`, by reporting E0504 when the
solver cannot find a principal type (§4.7).
This is a conservative rejection, not a completeness proof or a general
representation of relational intersection constraints. The cited calculi
and the finite/cofinite scalar component retain their separate arguments.**
A transfer argument for i must preserve solutions, rather than infer
principality merely from being a smaller language.

### What the references establish

[MLstruct, extended version 8.0](https://lptk.github.io/files/%5Bv8.0%5D%20mlstruct.pdf)
§5.4 states soundness (Theorem 5.5), termination of constraining (5.7), and
completeness with respect to scheme subsumption (5.8). Lemma 5.9 is the
solution-preserving constraining result. These statements concern its
specified calculus, with Boolean type connectives and its particular
subtyping rules; they do not establish completeness for an implementation
with additional constraints. Its §2.3.4 excludes intersection overloading.
Its §3.3 uses negation to handle variable-containing unions and intersections
without choosing a solution. Restricting the available types requires showing
that a principal result is still expressible in the restriction.

[Simple-sub](https://cse.hkust.edu.hk/~parreaux/publication/icfp20/)
provides the simpler bounds-and-levels account used by i. The local
`.rd/ref/simple-sub/shared/src/main/scala/simplesub/Typer.scala` has the
relevant `constrain`, `extrude`, and `freshenAbove` operations. The local
MLstruct `ConstraintSolver.scala` and `NormalForms.scala` additionally move
negations across constraints and construct Boolean normal forms. The
`.rd/ref/mlsub` directory was absent during this audit. Local reference paths
are relative to the coordinator's `.rd/ref`, not files needed to build i.

### The precise i restriction

[§4.2, “Literal types”](../spec.md#42-built-in-types) restricts scalar value
sets to all, finite, or cofinite subsets of `Int` and `String`.
[§4.4](../spec.md#44-union-intersection-and-difference) prescribes one member
per constructor, merging arguments by variance. Invariant arguments must be
equal or the merge is an error. All function types are one constructor;
their merge is the prescribed parameter/result merge, not an overload that
remembers a separate result for each domain. Closed data has a constructor
per shape ([§4.1](../spec.md#41-types-are-sets-of-values)); it is not
MLstruct's open-record system.

For scalars, take a finite support containing every literal mentioned in a
given expression. Boolean operations on its subsets and on their complements
remain finite or cofinite. Equality, inclusion, and disjointness are exact;
there is no need to widen a singleton to its scalar domain. The implementation
is [lits.rs](../../crates/ilang-check/src/types/lits.rs), `ValueSet::{union,
meet, minus, subset, disjoint}`; `None` from a set operation means empty,
which `Ty::with_values` turns into `Never`. This establishes the scalar
algebra lemma, not by itself the binding theorem.

i has no unrestricted complement type. Its internal
[`Ty`](../../crates/ilang-check/src/types/mod.rs) has `Union`, `Inter`, and
`Diff`, but no general negation. In particular:

- The right side of a difference cannot have a variable member. The left
  may remain symbolic as `a - U`.
- A non-scalar `T - U` with the same constructor removes that constructor
  and requires `T <: U`. It is not set subtraction on the constructor's
  arguments. `List(Int | String) - List(String)` must fail.
- A written union with multiple variable members is allowed only in output
  places; an intersection containing a variable only in input places.
  These restrictions apply after expansion and value-type substitution.
- Structural recursive inference types are rejected by §4.7, even though
  the reference calculus supports recursive types.

Enforcement is in [infer.rs](../../crates/ilang-check/src/types/infer.rs),
`check_places`, `conv_mode`, and `check_occurs`; and
[value_types.rs](../../crates/ilang-check/src/types/value_types.rs),
`check_value_places`. The algebra is implemented in
[solve.rs](../../crates/ilang-check/src/types/solve.rs), `union_in`,
`inter_in`, `merge`, `merge_arg`, `diff_mode`, `diff_left`, and `route`.
`route` rejects multiple residual variable choices rather than choosing one.

For the acyclic, rank-one, effect-free fragment, with facts fixed and the
above polarity restrictions, the intended argument is: generate structural
subtype constraints; decompose by the unique constructor; retain variable
bounds; generalize only variables absent from the environment. Scalar
normalization is lossless, as are the prescribed ground constructor merges.
What is **not shown** is a completeness lemma for *all* reachable open
constraints in this fragment. At the audit revision, `constrain_in` handled
a left intersection by trying its concrete part and then variable parts, keeping
the first successful variable bound. The probe below shows that the
existing place restriction does not make this choice lossless. A proof for
this implementation cannot simply assume that invariant; it needs an
algorithm correction or a separately authorized language restriction.

Candidate probes:

```i
// 01-base
id = (x) { x }
bounded = (x) {
	x + 1
	x
}
join = (xs : { List(Int) | List(String) }) { xs }
except_a = (s : String - "a") { s }
literals = [200, 404]
```

```i
// 01-polarity: attempts the excluded choice of variable for an input
bad = <A, B>(x : { A | B }) { x }
```

The stronger candidate puts a permitted variable intersection in an input,
then requires that intersection to be Int while retaining its components:

```i
// 01-intersection
choose = <A, B>(x : A & B, a : A, b : B) { (x + 1, a, b) }
```

`01-intersection-use` additionally has
`use = () { choose(1, "s", 1) }`; `01-intersection-swap` is that program
with just `A & B` changed to `B & A`. One satisfying assignment is
`A = 1 | "s"`, `B = 1`: their intersection is `1`, so `x + 1` is valid.
The swapped spelling has the same specified intersection.

Observed with the release compiler:

```text
01-intersection: exit 0
choose : (A & Int & B, A & Int, B) -> (Int, A, B)
01-intersection-use: exit 1, E0501
"s" is not a subtype of Int
01-intersection-swap: exit 0
choose : (A & Int & B, B, A & Int) -> (Int, B, A)
use : () -> (Int, 1 | "s", 1)
```

Shown variable names are assigned afresh; the second tuple position is
what matters. The original inferred scheme requires the second argument
to be Int, so cannot subsume the demonstrated valid typing which accepts
`"s"` there. The swapped scheme does. This is a counterexample to the
current compiler's principal inference/solution preservation, **not** a
proof that no principal scheme can exist in a better account of the
language. Neither annotations nor tests are changed to hide it.

**Correction (`685a1b04c`, `cde147079`, `c51d1d5a5`).**
The two operand orders now receive E0504 at
`x + 1`: the solver cannot infer a principal solution from the different
successful specializations. It first checks whether an existing bound
already proves the goal. It retains a successful structural constructor
proof, then tries competing variable alternatives in isolated states.
A unique remaining variable constraint keeps its ordinary diagnostic and
recovery behavior. No failed speculative trial leaves bounds behind. Each
read-only alternative gets its own proof budget, so an expensive earlier
member cannot prevent a later proof. An ambiguous subproblem is not discarded
as though it had no solutions. The definition-only probe also receives
E0504. This intentionally removes the old accepted,
nonprincipal schemes under §4.7's requirement to report an error where a
principal type cannot be found. It does not claim the satisfying assignment
above is invalid. Adding an explicit `Int` member to either intersection
admits the call without specializing `A` or `B`; both orders have positive
regressions. The two rejected orders and this positive case are fixtures
under `tests/spec/4.4/`, with checker coverage in `row_schemes.rs`.

The other base probes check successfully with `id : (A) -> A`,
`bounded : (A & Int) -> A`, `join : (List(Int | String)) -> List(Int | String)`,
`except_a : (String - "a") -> String - "a"`, and
`literals : List(200 | 404)`. `01-polarity` rejects with E0518, as §4.4
requires. This successful restriction check does not repair the intersection
counterexample.

## 2. Needs, row variables, and deferred meets

**Verdict: holds for merging fully determined, compatible entries, because
the merge is their unique prescribed meet; not shown for the full open-row
system. The missed deferred E0515 rejection below is fixed in `161d07271`,
with follow-ups `c9193f54c`, `cde147079`, `31e59eb2d` and `c51d1d5a5`;
this retains a soundness obligation for genuinely empty whole-service
meets. Decision 1 supersedes the earlier claim that the method-only
`Read(Never)` probe itself must be rejected.**

[§9.2, “Merging”](../spec.md#92-services) says that merging needs takes their
intersection. Its next paragraph makes an empty service intersection an
E0515 error when neither operand is `Never`. [§4.5 rule 4](../spec.md#45-subtypes)
compares needs in the reverse direction from values: a provider must be a
value subtype of the required service. [Syntax three §2.10.4](syntax-3.md#2104-typing-of-needs-review-rules-a-b-c)
states rules A and B; the latter keeps a row tail from smuggling in a second
form of a stated service.

This intersection is a conjunction of requirements on an environment:
an environment satisfies `Db & Log` when its Db entry satisfies Db and its
Log entry satisfies Log. It is not the ordinary value intersection of two
different nominal service constructors, which §4.4 would make `Never`.
Modeling a need as a constructor-indexed map of provider predicates makes
the different-key meet straightforward: retain both entries. Same-key
requirements intersect their admissible providers. A principality proof
must account for this separate sort and its row tails, rather than apply
the value lattice theorem to the surface `&` notation unchanged.

For a contravariant service `Emit(A) = (emit = (A) -> Nil)`, the meet of
`Emit(Int)` and `Emit(String)` is `Emit(Int | String)`. A provider must handle
both. For covariant `Read(A) = (read = () -> A)`, the meet of `Read(Int)`
and `Read(String)` is `Read(Never)`. Under Owner Decision 1 and §4.1,
a function that stops or diverges inhabits `() -> Never`, so this whole
service remains inhabited and §9.2 does not require E0515. In contrast,
`Read(A) = (value = A)` has no value at `Read(Never)` and its disjoint meet
must fail. An invariant service argument needs equality, not a choice
between forms. These ground operations introduce no alternative solution.

The implementation uses an internal row collection and **not** ordinary
value union for needs. In [solve.rs](../../crates/ilang-check/src/types/solve.rs),
`Mode::Env`, `merge`'s environment case, and `merge_arg` reverse the argument
operation as required. At the original audit revision, errors also had a
separately represented function component. The merged E3 implementation has
only the ordinary function need: `Fail(E)` is ordinary labelled service data
in that need, without a separate error component. Decision 2 retains the
`Fail(Never)` key; simplifying its argument does not provide the service.
The surface interpretation remains the intersection in §9.2.

For an undetermined covariant argument, `need_meet` makes a fresh variable
with *both* operands as upper bounds and retains `need_meet` metadata. It
does not equate the operands or decide that their meet is empty.
`add_bound` propagates later bounds, and `check_need_meets` tests conflicts
which become determined during completion; `Checker::check_occurs` invokes
the latter too. `extrude` and `freshen` copy an obligation on a surviving
variable. This does not prove it survives simplification and rebuilding at
generalization. Replacing it by a fresh unconstrained argument, equating its
operands, or retaining just the visible intersection would lose information.

The logical existence of a meet is weaker than the implemented requirement:
an instance can make a previously open meet empty, and must then fail
E0515. A full argument needs an invariant showing that every deferred meet
denotes exactly the original intersection and that every forbidden empty
instance is rejected, including through generalization and variable links.
Two upper bounds alone show containment in the meet, not that all solutions
of the meet survive later transformations. The code's obligation retention
and its unit test `deferred_need_meet_keeps_both_upper_bounds_and_reports_late_conflicts`
are evidence, not that proof.

For rows, [§9.2, “Row variables”](../spec.md#92-services) requires that in
`(..N, C(…))`, the tail has no `C`, and rejects providing an unstated key
through a rigid tail. `Solver::handle`, `diff_bound_mode`, `diff_left`, and
`add_bound` route same-key requirements to the handler and carry the
`handled` obligation across row inclusions. `hidden_handlers` and
`Checker::generalize` reject a scheme that would hide a required witness.
This is additional structure on a quantified row, not an unconstrained
MLstruct type variable. A scheme's printed arrow is insufficient evidence
that the obligation is retained. The no-skolem implementation limitation in
§5 below also prevents claiming the specified rigid-row case is covered.

Candidate probes:

```i
// 02-needs
Emit(A) = (emit = (A) -> Nil)
need_int = () <- (e : Emit(Int)) { e.emit(1) }
need_string = () <- (e : Emit(String)) { e.emit("s") }
both = () {
	need_int()
	need_string()
}
```

```i
// 02-empty
Read(A) = (read = () -> A)
need_int = () <- (r : Read(Int)) { r.read() }
need_string = () <- (r : Read(String)) { r.read() }
both = () {
	need_int()
	need_string()
}
```

```i
// 02-deferred; 02-deferred-use additionally has the last line
Read(A) = (read = () -> A)
read = <A>(x : A) <- (r : Read(A)) { r.read() }
both = <A, B>(x : A, y : B) {
	read(x)
	read(y)
}
// use = () { both(1, "s") }
```

```i
// 02-rows; 02-row-split replaces caught with wrong below
(
	fail = i/fail
)
body = () {
	fail.raise(1)
	fail.raise("s")
}
caught = () { fail.catch(body) }
// wrong: () -> Result(Never, Int) = () { fail.catch(body) }
```

The last candidate tries the erroneous split described in syntax three's
rule B: hiding the String error in the residual row instead of returning it.
Observed: `02-needs` checks with
`both : () <- (Emit(Int | String)) -> Nil`; `02-empty` rejects with E0515,
explicitly naming `Read(Int)` and `Read(String)` at that historical revision.
Decision 1 now accepts this method-only `02-empty` probe, as explained above;
only its required-value-field variant is empty. `02-rows` checks with
`body : () <- (Fail(1 | "s")) -> Never` and
`caught : () -> Result(Never, 1 | "s")`; `02-row-split` rejects with E0501
because `"s"` is not a subtype of `Int`.

Both original deferred probes check. In particular calling `both(1, "s")`
is **not** evidence of a missed empty meet: the arguments supply lower
bounds to flexible variables, which may grow to a common supertype. They
do not force the two required service arguments to be disjoint closed
types. A stronger deferred test below uses callback result and need
variance to impose the corresponding upper bounds as well.

```i
// 02-deferred-functions
Read(A) = (read = () -> A)
both = <A, B>(f : () <- (Read(A)) -> A, g : () <- (Read(B)) -> B) {
	f()
	g()
}
```

`02-deferred-functions-use` additionally has:

```i
need_int = () <- (r : Read(Int)) { r.read() }
need_string = () <- (r : Read(String)) { r.read() }
use = () { both(need_int, need_string) }
```

Matching the first callback's result contributes `Int <: A`; its need
comparison contributes `Read(A) <: Read(Int)`, hence `A <: Int`. The second
callback similarly fixes `B` to `String`. Unlike passing `1` and `"s"`
alone, these constraints force an empty meet after instantiation.

Observed: the generic definition checks with

```text
both : (() <- (Read(A)) -> A, () <- (Read(B)) -> B) <- (Read(A & B)) -> B
```

The instantiated program **also checked at the original audit revision**, showing
`use : () <- (Read(Never)) -> String`, with no E0515. This differs from the
ground `02-empty` rejection, despite neither constituent service argument
being `Never`. The original audit called this a missed rejection. Owner
Decision 1 supersedes that interpretation: a method-only `Read(Never)`
is inhabited, and this exact program must remain accepted. Replacing the
service by `Read(A) = (value = A)` and each `r.read()` by `r.value` tests
the genuinely empty deferred meet while retaining the generalization path.

A relevant audit boundary is `Checker::{generalize, rebuild}`:
`Solver::has_obligations` protects hidden differences/handlers, but does not
test `need_meet`. Rebuilding a displayed `A & B` does not recreate the
deferred meet's metadata. This is a candidate explanation for the observed
loss, not a traced causal diagnosis. The scalar/ground proof and the
`freshen` metadata-copying branch do not cover this path.

**Correction (`161d07271`, `c9193f54c`, `cde147079`, `31e59eb2d`,
`c51d1d5a5`).**
The traced loss was at
generalization: `has_obligations` ignored `need_meet`, so `generalize`
rebuilt the simplified intersection without its marked variable or service
forms. It now keeps the bounded scheme when a reachable variable carries
that obligation. Scheme snapshots also retain `has_need_meets`; copying
only the marked variables while dropping that flag disabled later checks.
The transfer encoding already copies the obligation and both operands.
Meet discovery also follows reverse inclusion links: a wider receiver can
otherwise hide a marked predecessor. Instantiation copies that connected
quantified graph with one map, stopping at nongeneralized variables and
keeping the receiver's own bounds distinct from the meet's bounds. Handler
discovery keeps its existing directed traversal. A regression checks a
conflict in a fresh receiver instance while the original quantified bounds
remain unchanged. Freshening retains input provenance for value variables
connected to a meet, so provisional lower bounds alone do not close them.
Equal concrete lower and upper bounds still determine an input. The
ordinary-value control passes in both need orders. Callback need fitting
preserves the late `NeedMeet` and
its service pair instead of replacing it with a generic form mismatch. The callback helper alone remains admissible, as do
compatible callback instances. After applying Decision 1, disjoint
method-only instances remain accepted, whereas disjoint required-value-field
instances report E0515 under §9.2. Fixtures under `tests/spec/9/` and checker tests cover the
open definition, compatible instances, local and imported disjoint uses,
retention through a receiving variable, snapshots, transfer and freshening.
No original golden error line was removed.

## 3. Value-type reads and fitted patterns

**Verdict: ordinary annotation reads hold under the restriction that a
generalized source is freshly instantiated and a monomorphic source stays
shared. Permanent reads hold only for the specified closed completion
result. The full composition with inference is not shown.** Known types and
inferred types must remain separate.

[§4.7, “Reading types of values”](../spec.md#47-type-inference) distinguishes
an annotation instance from a permanent type name. A permanent read cannot
choose a generalized instance to fit a pattern. Demanded weak parts are
closed from the union of incoming types, not from the reader's desired
answer; fixation adds equality constraints and keeps the previous bounds.
An unfilled part is an error, rather than an invented `Never`. After
completion, a later permanent read requires the whole source closed, even
when a pattern skips open arguments. Other modules cannot fix the source.

These restrictions prevent a type name from selecting one of several
instances and publishing it as if it described every use. For an already
closed source, the read is just substitution of a fixed type, so introduces
no inference choice. For a generalized annotation read, ordinary scheme
instantiation supplies the principality argument. For a shared monotype,
copying would be unsound; retaining identity supplies the same constraints
as an ordinary use.

Implementation: [value_types.rs](../../crates/ilang-check/src/types/value_types.rs),
`read_annotation` uses `instantiate`; `read_shape` erases unresolved
structure rather than treating temporary lower bounds as a known shape;
`Closure::{bounds, close, annotation}` computes closure; `finish_reads`
collects equations against one solver snapshot; `checked_batch` checks them
before publication; and `complete_ready_reads` processes dependent names.
`check_value_places` enforces polarity after reading. These mechanisms
implement a specified completion policy. They do not establish that its
closed solution is a principal solution of the original unconstrained
binding. Equality fixation deliberately excludes later widening. The
property to prove is principality *with the completion demands included*,
and independence from the order of unrelated groups under the conservative
earlier-weak-variable rule. Uniqueness of the chosen union alone is not a
proof of this property when invariance, upper bounds, and aliases interact.

[§4.8 rule 13 and the value-type paragraph](../spec.md#48-typing-rules)
explicitly allow a known type to be narrower than an inferred type. With
`x = "a" = s` and `s : String`, successful matching makes `"a"` a known fact,
but the inferred type of `x` is `String`. An annotation naming `x` reads
that inferred type. A key access uses a whole-scalar bound with fresh
noncovariant holes for inference; its known result is computed separately.
[known.rs](../../crates/ilang-check/src/types/known.rs), `fit_known`,
`known_bound`, and `scheme_known`, and
[infer.rs](../../crates/ilang-check/src/types/infer.rs), `data_part` and
`tuple_index`, implement these different operations. Preserving a useful
fact must not silently add
an equality to the inference variable.

For fitted patterns, §4.8's “To fit” paragraph gives a literal the whole
scalar type. A constructor pattern permits the residual `r - Q`, with
failure going through `Assert`; disjointness produces W0505, not rejection
or a fabricated successful match. [pattern.rs](../../crates/ilang-check/src/types/pattern.rs),
`fit`, `fit_in`, `pat`, and `binding_may_fail`, plus
`Checker::{binding_failure, warn_never_matches}`, distinguish these cases.
This preserves the intended constraint problem: a refutable binding does
not force every value of the source to have matched. The pending failure
checks in `Checker::group` run after group linking. A proof still needs to
show that deciding failure at that point preserves all admissible need
solutions. W0505 is diagnostic evidence about matching, not a proof of
principality.

Candidate probes:

```i
// 03-reads
id = (x) { x }
a : id = (n : Int) { n }
b : id = (s : String) { s }
copy = (s : String) {
	x = "a" = s
	p = (x, 0).0
	q : p = "b"
	q
}
```

The two annotation reads try independent instances. The final annotation
tries to detect an accidental use of the narrower known literal as the
inferred type of `p`.

```i
// 03-open-read: a permanent name must not choose an identity instance
id = (x) { x }
Alias = id
```

```i
// 03-fitted: an impossible fitted constructor is a warning and Assert need
Ok(A) = (A)
Error(E) = (E)
f = () {
	r = Error("no")
	Ok(v) = r
	v
}
```

The next candidate exercises actual weak fixation, rather than only a
generalized read:

```i
// 03-fixation
Inv(A) = (id = (A) -> A)
make = (x) { Inv(id = (y) { x }) }
source = make(1)
Inv(Element) = source
```

`03-fixation-use` adds `use = () { source.id(2) }`;
`03-no-fixation` has that use but omits `Inv(Element) = source`.
The argument of `Inv` is invariant. A permanent read fixing it to `1`
must reject a later use that would widen it to include `2`; without the
read, the one shared weak argument may accumulate both values. Such a
difference is explicitly required by §4.7, not a counterexample to the
specified completion policy.

Observed: `03-reads` checks with `a : (Int) -> Int`,
`b : (String) -> String`, and `copy : (String) <- (Assert) -> String`.
Thus the generalized annotation reads are independent and the fitted
literal is not used as the inferred type read by `q: p`.
This first probe alone does not show that the narrower known fact survived
generalization: §4 exposes that separate issue. `03-reads-call` repeats
`03-reads` with only `x = "a" = s` replaced by
`x = "a" = id(s)`, making the fitted binder nondirect and retaining that
fact. This stronger check also exits 0 with the same types, including
`copy : (String) <- (Assert) -> String`.
`03-open-read` rejects with E0533, “a permanent alias cannot contain a
generalized variable or an unknown type”. `03-fitted` checks with
`f : () <- (Assert) -> Never` and W0505 naming the impossible Ok pattern.

`03-fixation` checks with `source : Inv(1)`. Adding the later use rejects
with E0501, `2` not a subtype of `1`, and a note at the source's fixation
read. Omitting that read checks with `source : Inv(1 | 2)` and
`use : () -> { 1 | 2 }`. These different outcomes implement the specified
extra completion constraint. Type text printed from the rejected program
is recovery output, not a successfully changed permanent alias.

## 4. Shape groups and selection tables

**Verdict: the design reduces to ordinary constraints under a fixed, sound
known-type environment and selection table, conditional on the solver.
The current checker does not fully implement the specified known-type
construction or selected domains. A principal ordinary arrow alone does
not describe this feature; whole-language completeness is not shown.**

[§6.12, “Selection”](../spec.md#612-branches-and-matching) and
[§4.8 rule 5a](../spec.md#48-typing-rules) prescribe the choice before
solving. [Syntax three §2.6.2](syntax-3.md#262-selection-at-a-direct-call)
is explicit about the **per-key** test: skip a branch if some known key is
disjoint from what that branch can still take at that key. Earlier guarded
branches take nothing. Unknown keys exclude nothing. This is not arbitrary
correlated overload resolution or a search for whichever branch makes the
result annotation succeed.

For fixed facts the selected set is uniquely determined. Once that set is
chosen, the parameter and result unions and the shared need define the
constraint problem. There is no alternative branch choice for inference
to guess. [Syntax three §2.6.6](syntax-3.md#266-what-selection-guarantees)
gives the soundness argument: a first-matching branch cannot have been
skipped if every runtime argument value belongs to its known type.
[§4.8's opening paragraph](../spec.md#48-typing-rules) requires precisely
that fact invariant. Soundness of this exclusion argument does not prove
principality of the subsequent constraints.

Enforcement: [groups.rs](../../crates/ilang-check/src/types/groups.rs),
`keep_branches`, `branch_table`, `select`, `skipped`, `key_disjoint`,
`selected_fn`, `select_at_call`, and `select_by_expected`;
[coverage.rs](../../crates/ilang-check/src/types/coverage.rs) supplies
usefulness/taken tests. The table generalizes the group function and the
branch results together (`BranchTable::combined`) so that the selected
result is not instantiated independently of its arguments. `infer_k`
applies selection by an expected type. `call` gathers the argument facts
before constraining the selected call. Expected facts for higher-order
arguments are governed by §4.8 and `known.rs::{match_k, whole_sigma}`,
including making hidden inputs whole rather than fixing them from one
initial literal.

There is also a concrete implementation question: `selected_fn` currently
copies `f.param` from the whole group while narrowing only `ret`.
The specified selected-domain check needs a separate justification or
implementation. Syntax three §2.6.6 expressly says it may reject a
runtime-correct program whose known argument is narrower than its inferred
type. This probe isolates that distinction:

```i
// 04-known-domain
show = ("a") { "a" } | (other : String) { 1 }
f = (s : String) {
	x = "a" = s
	show(x)
}
```

The fact `K(x) = "a"` selects the first branch; the inferred `String` is
not a subtype of its selected domain `"a"`. Checking the whole group's
`String` domain would instead accept it. This probes a spec/implementation
gap, distinct from nonexistence of a principal type.

An untyped direct alias preserves the table. A value in data, a parameter,
or a binding with a written type does not. The ordinary arrow for `show`
therefore cannot serve as a complete interface for every source-level use
of its name: its table is extra static evidence. Comparing `show(1)` with
an eta-expanded wrapper is not a counterexample; those expressions have
different facts under the prescribed rule. A binding theorem must quantify
over typings with that evidence fixed. It must also show that obtaining
expected facts never picks one among otherwise admissible inference
solutions, that tables survive publication/aliases correctly, and that the
known-fact invariant holds for all the constructions of §4.8. None of
these properties follows merely from deterministic per-key selection.

Candidate probes:

```i
// 04-selection
show = (n : Int) { "int" } | (s : String) { 1 }
direct = show(1)
alias = show
by_alias = alias(1)
h = (x) { show(x) }
wrapped = (show, 0)
ordinary = wrapped.0(1)
shapes = (n : Int) { "one" } | (n : Int, m : Int) { True }
shaped = shapes(1, 2)
```

```i
// 04-fallback: an unknown parameter must not select only the Int branch
show = (n : Int) { "int" } | (s : String) { 1 }
h = (x) { show(x) }
wrong : String = h(1)
```

Observed: `04-selection` checks with `direct : "int"`,
`by_alias : "int"`, and `h : (Int | String) -> { "int" | 1 }`.
`ordinary : { String | Int }` is deliberately broader: accessing the stored
function through `.0` uses the whole-scalar inference bound of §4.8 rule
13. The shape-selected `shaped` is `Bool`. `04-fallback` rejects with E0501
because the String annotation does not admit the `1` result.

`04-known-domain` checks, showing
`f : (String) <- (Assert) -> { "a" | 1 }`. It does not even retain the specified
literal fact strongly enough to narrow the result here. In `local_binding`,
a direct source is generalized and `scheme_known` recomputes its known
type from its String scheme. Thus this first probe does not isolate the
selected-domain check. The follow-up below uses a nondirect binding value
to preserve the literal fact through that step.

```i
// 04-known-domain-call
identity = (x) { x }
show = ("a") { "a" } | (other : String) { 1 }
f = (s : String) {
	x = "a" = identity(s)
	show(x)
}
```

Observed: `04-known-domain-call` checks with
`f : (String) <- (Assert) -> "a"`. Here the narrowed result confirms the
literal fact is used for selection. Acceptance nevertheless omits the
specified `String <: "a"` selected-domain constraint. This agrees with the
`selected_fn` code copying the whole group's parameter. Thus this probe
does isolate the domain-checking gap, while the preceding one additionally
exposes loss of a known fact at a direct fitted binder.

## 5. Explicit polymorphism and level escape

**Verdict: ordinary function `<A>` follows the rank-one argument under the
same solver restrictions; not shown for written polymorphic parameter
types. Their specified skolemization is not implemented by this revision's
syntax-three path.**

[§4.7, “Type parameters”](../spec.md#47-type-inference) makes two distinct
promises. A parameter on a function expression is an inference variable
which may become more exact: the spec explicitly permits
`id = <A>(x : A) -> A { 1 }`. This does not assert an arbitrary, rigid `A`.
A written polymorphic type in a parameter, data, definition, or annotation
instead needs fresh rigid parameters that cannot escape, and each use
instantiates it anew. Inference never guesses a polymorphic parameter type.

For the first case, naming a flexible variable introduces no new choice.
[infer.rs](../../crates/ilang-check/src/types/infer.rs), `new_tyvars` and
`conv_mode`, creates named input variables. `instantiate` and
[solve.rs](../../crates/ilang-check/src/types/solve.rs), `freshen`, copy
generalized variables above the scheme level. `extrude` lowers variables
connected to an outer level while retaining bounds, preventing independent
generalization of something shared with the environment. This is level
discipline, not skolem escape checking.

Checking an explicitly given universal type could preserve principality
relative to that annotation: introduce skolems, prove the body at their
fixed identities, prevent them entering outer types, then package the
universal type. The annotation avoids guessing quantifier placement. The
proof would still need higher-rank subsumption, fresh instantiation of a
parameter's scheme at every use, variance, and row-lacks obligations.
Ordinary extrusion is insufficient: flexible variables may acquire bounds;
skolems may not.

The actual restriction is explicit in
[syn3/lower.rs](../../crates/ilang-syntax/src/syn3/lower.rs), `Cx::fn_type`:
generics on a function type other than the type of a binding report that
stage 7 is unavailable. `Cx::chain` similarly rejects generic function
literals outside a binding value. A binding's outer generics are lifted to
its binding parameters. Active HIR `TypeKind::Fn` and inference `Ty::Fn`
have no universal binder. `Solver::freeze` serves macro snapshot proofs;
it is not evidence that written polymorphic parameters get skolems.
Neither higher-rank principality nor its specified escape restriction is
established here. This coverage gap is not a counterexample to principality
of the intended annotated calculus.

Candidate probes:

```i
// 05-flexible
id = <A>(x : A) -> A { 1 }
constant : <A>(A) -> A = (x) { 1 }
a = id("s")
b = constant("s")
```

The function-head case is explicitly permitted. The annotation case tests
how outer written generics are treated, not a premise of rigid checking.

<!-- old-syntax-check: E0101 -->
```i
// 05-rank: attempts the specified polymorphic-parameter behavior
use = (f : <A>(A) -> A) { (f(1), f("s")) }
id = (x) { x }
result = use(id)
```

```i
// 05-escape: tries to generalize a captured outer value through a local
escape = (x) {
	local : <A>(A) -> A = (y) { x }
	local
}
result = escape(1)("s")
```

The last probe tests implemented levels, not skolem escape: the local's
result must stay tied to the captured value.

Observed: `05-flexible` checks with both `id` and `constant` shown as
`(A) -> A | 1`, and both calls as `"s" | 1`. This confirms flexible outer
variables; it does not demonstrate the rigid annotation rule.
`05-rank` rejects with E0101 and the exact stage-7 diagnostic described
above, before skolem checking could occur. `05-escape` checks with
`escape : (A) -> (B) -> { B | A }` and `result : { "s" | 1 }`: the captured outer
`A` remains present. Its extra `B` alternative comes from the flexible
identity-shaped annotation, not a successful higher-rank escape proof.

## 6. Typed macros and observing types

**Verdict: holds under signature-only inference, one unchanged completed
group snapshot, and read-only expansion checking, conditional on the
inference and observation rules. Not shown: invariance of every query
under equivalent principal constraint representations.**

[§14.8's second and “Expansion” paragraphs](../spec.md#148-typed-macros)
address the intermediate-type concern: infer the whole group against
signatures, expand afterwards against one snapshot, and require output to
fit the instantiated signature without adding snapshot bounds. Directness
and generalization are decided before expansion. A generic binding expands
once. A variable reachable from a generalized, declared, published or
top-level weak variable is fixed for queries. Only unreachable local closed
variables receive the specified union-of-lowers or intersection-of-uppers
observation. Other variables stay opaque; `subtype` may return `NotKnown`.

Under these restrictions a successful expansion cannot specialize the
surrounding scheme: erasing its implementation leaves the same signature
constraints. A fixed generic input cannot be replaced by a future instance.
A bound-based observation of a closed local is not permission to mutate
its constraints or a published scheme. Expansion may still reject a call.
This establishes the intended conservativity argument, conditional on its
premises, not completeness or a principal semantics for arbitrary generated
code.

Enforcement in [infer.rs](../../crates/ilang-check/src/types/infer.rs):
`group` links values, completes coverage and value reads, builds fixed
reachability roots, then runs `expand_typed_calls` before publication.
That function clones the snapshot, calls `freeze` on a trial solver,
checks output/result/needs against the signature and exposed query result,
and checks old variable states are unchanged.
[typed_macro.rs](../../crates/ilang-check/src/types/typed_macro.rs),
`reachable`, `observe`, and `TypedMacroQuery::{new, type_of, signature,
subtype, members}`, implements observations. `type_of` reuses substitutions
within a run. `subtype` uses a frozen trial, returning `No` only on closed
types and otherwise `NotKnown` on failure. [solve.rs](../../crates/ilang-check/src/types/solve.rs),
`freeze`, `rigid_constraint` and frozen-variable guards block new evidence
on old variables.

Thus the design does not run queries halfway through an arbitrary inference
sequence. But a snapshot is a constraint graph, not a proved canonical
principal type. Equivalent graphs must give equivalent answers, including
`members`, fixed-variable intersections/differences and invariant sharing.
Different layouts of bounds must not change a semantic subtype question
from `NotKnown` to `Yes`, or change a closed variable's members. Completion
also precedes later groups constraining weak top-level values; keeping
those variables opaque is essential. Freezing alone does not prove this
observational property.

All three probes use this `src/macros.is`. The macro tries to choose an Int
expansion from a query, rather than merely forwarding an argument:

```is
(, (= syntax (/ i syntax)) (= fail (/ i fail)))
(, m)
(@ macro (: m (-> [A] (, A) Int)))
(@ macro
  (= m
    (-> (, value) _
      {(= t ((. syntax type_of) value))
       (|> ((. syntax subtype) t `Int)
           (| (-> (, (. syntax Yes)) _ `7)
             (-> (, _) _
               ((. fail raise)
                ((. syntax SyntaxError)
                 (= message "query could not prove Int")
                 (= at value))))))})))
```

```i
// 06-closed: src/main.i
(
	m = princ/macros
)
value = m.m(42)
```

```i
// 06-generic: src/main.i
(
	m = princ/macros
)
f = (x) { m.m(x) }
value = f(42)
```

```i
// 06-weak: src/main.i
(
	m = princ/macros
)
maker = () { (x) { x } }
weak = maker()
f = () { m.m(weak(42)) }
```

The generic candidate tests whether a later call can specialize an earlier
run; the weak candidate tests whether one use supplies the final
whole-program type.

Observed: `06-closed` checks with `value : Int`, the signature's result
rather than the expansion's literal type `7`. `06-generic` and `06-weak`
both reject with E0234, the macro's “query could not prove Int” error, and
a note that the query returned `NotKnown`. The argument `42` neither
specializes the earlier generic macro run nor permits a query to close a
published weak variable. These probes did not expose a changing
intermediate type. They also do not establish representation invariance.
A representation-dependent rejection would be a separate stability defect;
it would not by itself show two incomparable types for an accepted caller.

## 7. Direct values and whole-program weak types

**Verdict: generalization holds under the direct-value and level/environment
restrictions, conditional on principal constraint solving. A nongeneralized
top-level value has a shared whole-program constraint problem, not an
independently instantiable scheme. The complete interaction with fixation
and macros is not shown.**

[§6.10's “direct value” paragraph](../spec.md#610-blocks-and-bindings)
defines the syntactic class: functions/chains, literals, names, and the
specified constructions entirely from direct values; data has no spread.
[§4.7, “Generalization”](../spec.md#47-type-inference) quantifies just
these values and just variables absent from the outside environment. Its
reason is that evaluating other expressions can produce a value specific
to one type, for example through `control.suspend`. It also requires one
type across uses of a weak binding and monomorphic recursion within a
group. [§4.8 rule 14](../spec.md#48-typing-rules) repeats the restriction.
Safety of that restriction and principality within it are separate claims.

Given a principal graph with environment identities fixed, quantifying
exactly eligible variables preserves the most general scheme: permitted
instances correspond to its solutions and other variables remain shared.
Directness is syntactic, so no purity inference is needed. `maker()` is not
generalized even when it returns an identity; a direct alias of its result
must not quantify the inherited weak variable.

Enforcement: [syn3/lower.rs](../../crates/ilang-syntax/src/syn3/lower.rs)
and [resolve.rs](../../crates/ilang-check/src/resolve.rs) retain binding
directness. [infer.rs](../../crates/ilang-check/src/types/infer.rs),
`local_binding`, raises the level only for direct values and generalizes
after fitting/occurs checking. `group` uses monomorphic holders during
mutual inference and generalizes afterwards. `Scheme::mono` and
`instantiate` retain weak identities. `generalize` keeps unsimplified
bounds for invariant sharing and recognized hidden difference/handler
obligations, and rejects hidden handler witnesses. The original deferred-meet
counterexample in §2, now fixed, shows why checking that retention is
necessary; the correction does not establish a theorem for every obligation.
[solve.rs](../../crates/ilang-check/src/types/solve.rs),
`extrude` and `freshen`, enforce levels and preserve row/meet metadata.

Both an Int and a String use can contribute to one weak argument type;
the first use must not freeze it unless a specified read demands fixation.
A displayed union of lowers is not universal quantification.
[§4.7, “Reading types of values”](../spec.md#47-type-inference) supplies
the extra completion rule when a read demands closure. It can legitimately
make later widening fail: adding the demand changes the constraint
problem. A later module cannot choose the completion result. The remaining
proof is solution preservation for whole-program sharing with completion
barriers, row obligations and macro snapshots.

Candidate probe:

```i
// 07-weak
direct = (x) { x }
maker = () { (x) { x } }
weak = maker()
alias = weak
a = weak(1)
b = alias("s")
c = direct(1)
d = direct("s")
```

`07-weak-reverse` swaps just the `a` and `b` lines. The weak value and its
direct alias must stay shared, while the direct identity's uses remain
independent. This tests two orders, not arbitrary group schedules.

Observed: both orders check. `weak` and `alias` each have
`(1 | "s") -> 1 | "s"`; both `a` and `b` have `1 | "s"`, while
`c : 1` and `d : "s"`. The reverse probe prints `"s" | 1` instead of
`1 | "s"`, which is the same union by §4.4. No type-order difference was
observed, and the direct alias did not generalize the inherited weak type.

## Reproducing the probes

Each probe is an independent package with `i.toml` containing
`name = "princ"` and the shown program in `src/main.i`; macro packages also
have the shown `src/macros.is`. Commented extra/replacement lines above
denote separate packages. Scratch packages and outputs are under ignored
`target/principality-probes/` in the audit worktree. No test expectations
were changed.

Build and check serially from the worktree:

```sh
H=/home/agent/Programming/i/.rd/cache/heavy
$H direnv exec /home/agent/Programming/i cargo build -p ilang-cli --release --locked
$H ./target/release/i check --types --project target/principality-probes/01-base
# Repeat the second command for each probe ID above, one at a time.
```

Checking on 2026-11-14 used the worktree's release binary without
copying it. The first wrapped release build exited 101 because the pinned
Binaryen submodule was missing. A reference clone could not use the shallow
local reference; `git submodule update --init --depth 1
crates/ilang-binaryen/binaryen` then initialized the pinned commit
`fba5005a132d85d578be5bd010e0487ed8e2ce3e`. Retrying the same wrapped
release build exited 0. No tracked compiler or dependency revision changed.

The initial 28 checks and three focused follow-ups ran serially under a
single wrapper slot per batch; the final value-read control used the
wrapper directly. The local capture commands were:

```sh
$H python target/principality-probes/run.py
$H python target/principality-probes/run.py 02-deferred-functions 02-deferred-functions-use 04-known-domain-call
$H ./target/release/i check --types --project target/principality-probes/03-reads-call
```

The capture driver is scratch tooling. On another worktree, recreate each
shown package and use the direct `i check` command above for each ID;
no driver is necessary. Each row below identifies that command's final
project directory. Exit 0 means accepted by the current compiler, including
the documented spec gaps; it does not mean the program agrees with the
spec. Exit 1 means a compiler diagnostic, not a crashed probe.

| Probe directory | Exit | Diagnostics | Observation |
|---|---:|---|---|
| `01-base` | 0 | W0502, W0504 | Base types and literal preservation |
| `01-intersection` | 0 | W0504 | Chooses first variable for the Int bound |
| `01-intersection-swap` | 0 | W0504 | Same specified intersection; call accepted |
| `01-intersection-use` | 1 | E0501 | Loses a valid typing; intersection witness |
| `01-polarity` | 1 | E0518 | Specified annotation restriction enforced |
| `02-deferred` | 0 | W0504 | Flexible lower bounds remain open |
| `02-deferred-functions` | 0 | W0504 | Open meet in a generalized callback helper |
| `02-deferred-functions-use` | 0 | W0504 | Required E0515 missing; Read(Never) accepted |
| `02-deferred-use` | 0 | W0504 | Lower bounds alone do not force empty meet |
| `02-empty` | 1 | E0515 | Ground empty meet rejected |
| `02-needs` | 0 | W0504 | Contravariant service meet |
| `02-row-split` | 1 | E0501 | Cannot hide String error in residual need |
| `02-rows` | 0 | W0504 | Both error types retained and handled |
| `03-fitted` | 0 | W0504, W0505 | Impossible fit retains Assert and warns |
| `03-fixation` | 0 |  | Invariant source fixed to 1 |
| `03-fixation-use` | 1 | E0501 | Later widening rejected with fixation note |
| `03-no-fixation` | 0 | W0504 | Undemanded weak argument can grow |
| `03-open-read` | 1 | E0533 | Permanent read cannot choose generic instance |
| `03-reads` | 0 | W0504 | Separate annotation instances and inferred read |
| `03-reads-call` | 0 | W0504 | Nondirect fitted control; inferred read preserved |
| `04-fallback` | 1 | E0501 | Unknown argument does not select Int branch alone |
| `04-known-domain` | 0 | W0504 | Accepted; direct fitted literal fact lost |
| `04-known-domain-call` | 0 | W0504 | Selected-domain check omitted |
| `04-selection` | 0 | W0504 | Direct alias, fallback, stored value, shape group |
| `05-escape` | 0 | W0504 | Captured outer variable remains in result |
| `05-flexible` | 0 | W0504 | Outer generics treated flexibly |
| `05-rank` | 1 | E0101 | Written polymorphic parameter unsupported |
| `06-closed` | 0 | W0504 | Query proves Int; output checked against signature |
| `06-generic` | 1 | E0234 | Query sees fixed generic variable |
| `06-weak` | 1 | E0234 | Query sees fixed published weak variable |
| `07-weak` | 0 | W0504 | Weak value and alias share one type |
| `07-weak-reverse` | 0 | W0504 | Same types modulo union ordering |

There were 32 completed checks: 22 accepted and 10 rejected. W0504 is the expected unused-top-level warning in these small
packages; W0502 is the deliberately unused `x + 1` in the base example.
W0505 is the fitted-pattern warning under investigation. Diagnostics and
types were inspected, not blessed into changed expectations.

The documentation check validates local links/anchors, balanced fences,
seven sections, every probe ID, and the final unproven list;
`git diff --cached --check` checks the staged documentation diff. No runtime
probe, full suite, unfiltered corpus, std check, or benchmark was run.

The obligations below concern the intended compiler requirement. Finite
probes cannot promote them to established guarantees. §4.2's literal
paragraph also says “inference still finds principal types”; its scalar
closure argument should be read with this limitation. This lane's spec
edit is deliberately confined to §4.7.

### Historical correction checking (2026-11-14)

This ledger predates Owner Decision 1 and the merge of `a02d34100`. Its
method-only rejection claims and older corpus interpretations are historical;
the 2026-11-15 merge ledger supersedes them. Main has since adapted the
alias criterion and the affected review fixtures to the owner decisions.

The wrapped checker run on `c51d1d5a5` completed at 06:53 with **325 passed,
4 existing stress tests ignored, and only
`spec_corpus_reverse_order_and_recovery` filtered out**. Every existing
golden, including every error line, is unchanged. The focused `deferred`
row-scheme filter passed both tests: the local/imported callback rejection
and the lower-bound-only acceptance control. `cargo fmt --all -- --check`
and both unstaged/staged `git diff --check` checks passed.

Two test-only clippy suggestions were then applied without changing the
assertions' meaning or compiler logic. The unit/clippy retry and release CLI
build passed. The positive spec fixtures received executable roots in
`3e475c4c4`: the bounded intersection fixture asserts both orders' results,
and the two type-only need controls have empty entries. The checker still
tests the lower-only probe without that harness entry. The final test-only
formatter correction is `967d9f2ba`; it changes no assertion or compiler logic.

Every heavy command below ran through `/home/agent/Programming/i/.rd/cache/heavy`.
Cargo commands also used `direnv exec /home/agent/Programming/i`; the wrapper
selects this worktree's musl development environment.

| Command | Result |
| --- | --- |
| `cargo test -p ilang-check --test row_schemes --locked deferred` | 2 passed: callback and lower-only controls |
| `cargo test -p ilang-check --locked -- --skip spec_corpus_reverse_order_and_recovery` | 325 passed, 4 existing ignored, 1 requested filter |
| `cargo test -p ilang-check --lib --locked` | 72 passed, 4 existing ignored after test-only clippy cleanup |
| `cargo clippy -p ilang-check --all-targets --locked -- -D warnings` | Passed |
| `cargo build -p ilang-cli --release --locked` | Passed |
| `cargo test -p ilang-cli --release --test spec --locked -- 4.4/` | 5/5 passed at O0/O2 |
| `cargo test -p ilang-cli --release --test spec --locked -- 4.5/` | 2/2 passed at O0/O2 |
| `cargo test -p ilang-cli --release --test spec --locked -- 9/` | 153/153 passed at O0/O2 |
| `./target/release/i test --project std --opt 0` | 2,171 passed, 0 failed |
| `./target/release/i test --project std --opt 2` | 2,171 passed, 0 failed |
| `python3 scripts/qa/corpus.py run --filter review-k4 --jobs 2` | Interrupted with exit 143 while isolating post-test wall-time waits; partial observations preserved |
| Same corpus filter with `--compile-cache target/pfix/compile-cache` | All 1,225 completed: 908 checker acceptances, 317 rejections, no resource failures; exit 1 on 865 saved-output differences and three requirement failures |
| `scripts/test-generation --mode short --binary ./target/release/i` | 32 base cases (20 accepted, 12 model rejections), 64 permutations, 32 annotation removals; no failures |
| `cargo fmt --all -- --check` | Passed after the test-only layout correction |
| `python3 -m py_compile scripts/qa/corpus.py` | Passed |
| Corpus filter `review-k4d/_loose/p/b10` with the same cache and two jobs | 1/1 passed after the assertion-guard correction, with both E0505 and E0515 retained |
| Checker comparison against starting commit `0c4942e1f` | All 1,225 corpus cases have identical complete diagnostics and shown root types; all match current CLI observations; nine audit controls have the intended outcomes |

The focused/clippy retry after the fixture entry changes also passed.
The first corpus attempt was interrupted to isolate post-test wall-time
timeouts: one process had printed its complete passing result but remained
alive for over 35 wall seconds with about one CPU second used. The CLI runs
global cache maintenance after commands. Its partial observations are
preserved. The completed rerun used a worktree-local compile cache with the
same saved bounds, two jobs, binary and assertions. Every recorded timeout
disappeared. Generation used that cache too. No expected observation was
changed.

The raw saved-output comparison is **not green**. It has 589 cases changing
only shown types, 267 changing diagnostics or runtime text, and nine changing
phase sets or exits. Much of the display/wording difference is the inherited
E3 removal of synthetic `Fail(E)` from opaque needs and E5 removal of provider
plans, already described in the [E6 integration audit](e6-main-corpus-audit.md).
Sixty-one changed observations exactly match newer saved observations of
identical source files and manifests elsewhere in the selected corpus.

The seven changed checker exits against the older observations are:

- `review-k4d/_loose/fuzz/e0505/21_135` and
  `review-k4-lane/alias_kind_error`: rejected before, accepted now.
- `review-k4s/mixed_recursive_0_0`, `mixed_recursive_0_17`,
  `mixed_recursive_1_0`, `mixed_recursive_1_17`: accepted before, now E0225
  for treating an environment row as ordinary data (§9.4).
- `review-k4s/std`: the vendored earlier std snapshot now reports five type
  errors. The current std project passed all 2,171 tests at each level.

Additionally, `review-k4s/min_main_3_0` and `min_main_5_0` now reach their
entries: their unused local closures no longer leave a need on `main`.
The starting checker produces the same diagnostics and shown types for all
these cases. They predate pfix; no corpus checker acceptance changes between
`0c4942e1f` and this correction. The baseline comparison does not separately
execute the old runtime. Existing baseline files remain byte-for-byte intact.

Two requirement failures were a runner contradiction, corrected separately
in `3d9fdec57`. `b10` explicitly requires both a structural-recursion E0505
and its provider E0515, but the runner unconditionally forbade E0505 while
checking either requirement. It now permits E0505 only when an applicable
requirement explicitly requests it; each requirement still needs its own
code. Only `b10` has such a requirement. Its replay passes without changing
either diagnostic line or any expectation. This follows §4.7's recursive-type
paragraph and retains §9.2's independent provider check.

The remaining `alias_kind_error` requirement asks for E0515 on a program
whose alias is `Row(N) = () <- (Fail(N)) -> Nil` and whose ordinary values
have types `Svc(Int)` and `Svc(Nil)`, with covariant `Svc(A) = (() -> A)`.
Here `N` is an error-payload type. §9.3 says “An error may be a value of any
type”; §4.4 merges these value types as `Svc(Int | Nil)`. It is not an opaque
service row, and its payload constructors are not service keys under `<-`.
The old rejection criterion needs review against those paragraphs; it has
not been weakened or rewritten to claim a green corpus.

The comparison with `0c4942e1f` uses an exact source snapshot of the checker,
with unchanged package/syntax dependencies and the same package graphs for
both versions. It compares complete diagnostics and shown root types, and
checks the current version against the completed CLI observations. Its nine
extra controls cover local/imported disjoint callbacks, open/lower-only
needs, both intersection orders with and without a call, and the explicit
`Int` positive case. A temporary-package lockfile setup failure was resolved
by generating that package's lockfile offline; the workspace lockfile is
unchanged.

The initial comparison completed all 1,234 inputs: both checkers agreed on
all 1,225 corpus inputs, but nine current results did not match the CLI
because the harness had not expanded their top-level test macros. That
validation failure was retained, not ignored. A second comparison expanded
those nine with the unchanged macro implementation, passed the same parsed
ASTs and hygiene to each checker, and again compared the complete output.
All nine matched both the starting checker and the current CLI. No typed
macro driver was shared between checker versions. Thus all 1,225 corpus
comparisons are validated against the real CLI; none changes checker
acceptance, diagnostics, or shown root types relative to the starting tip.
The comparison harness and exact records are under ignored `target/pfix/`.

The nine extra controls establish the intended changes separately:

| Control | Starting checker | Corrected checker | Rule |
|---|---|---|---|
| Disjoint callback needs, local | Accepted | E0515 | §9.2 deferred empty meet |
| Disjoint callback needs, imported | Accepted | E0515 | §9.2 through scheme transfer |
| Open callback helper | Accepted | Accepted | §9.2 undetermined meet retained |
| Lower-bound-only ordinary values | Accepted | Accepted | §4.4 value unions remain possible |
| Original intersection, with call | E0501 at call | E0504 at definition | §4.7 principal type requirement |
| Original intersection, definition only | Accepted | E0504 | §4.7 principal type requirement |
| Swapped intersection, with call | Accepted | E0504 | §§4.4, 4.7 same intersection |
| Swapped intersection, definition only | Accepted | E0504 | §§4.4, 4.7 same intersection |
| Explicit `Int` intersection member | Accepted | Accepted | §4.5 existing subtype proof |

The raw corpus command remains red against its old saved outputs and the
unchanged alias criterion. This baseline comparison establishes the scope of
pfix; it does not re-record those outputs or declare that criterion passed.

The full workspace, unfiltered spec suite, unfiltered corpus and
`scripts/checks` are reserved for the coordinator and were not run here.

## Unproven parts

1. Solution-preserving constraint decomposition for i's restricted open
   union/intersection/difference normal forms and closure under
   substitution. The concrete left-intersection counterexample now receives
   the required principality error rather than an arbitrary specialization;
   general relational intersection constraints remain unsupported.
   MLstruct's theorem does not transfer automatically to a negation-free
   implementation.
2. A principal representation for directed service rows with lacks/handled
   obligations, invariance and deferred meets; rejection of all forbidden
   empty meets in later instances. The required-value-field variant reports
   E0515 after the obligation-retention corrections; method-only
   `Read(Never)` remains inhabited under Decision 1. The finite
   regressions do not establish the full invariant for every transformation.
3. Principality relative to read-completion demands, retaining sharing
   through aliases, invariant positions and read batches; unrelated-group
   order independence under the conservative dependency rule. Fitted
   failure/coverage effects must preserve these solutions.
4. A binding judgment including known facts and selection tables, with
   proofs of the fact invariant and expected selection preserving all
   admissible typings. The demonstrated direct-fitted fact loss and omitted
   selected-domain check need reconciliation with that judgment.
5. The annotated higher-rank calculus, rigid instantiation and skolem/row
   escape checking. This path does not implement the specified polymorphic
   parameter types.
6. Observational equivalence of query snapshots for equivalent constraint
   graphs, including opaque weak variables, member enumeration and
   result-only variables; conservativity of all successful expansions.
7. The combined whole-program theorem for weak values, generalization,
   completion and snapshots. A printed monotype alone does not specify the
   shared constraint problem.


### Main integration (2026-11-15)

Merged `a02d341005c7329391bfd2941f4e16faeac216b9` after fetching origin.
The three conflicts were resolved as follows:

- `row_schemes.rs`: retained all main tests, then reapplied the three lane
  tests. Decision 1 changes the method-only callback probe to an acceptance
  control, locally and across modules. The deferred rejection probe now uses
  `Read(A) = (value = A)` with disjoint Int/String fields. The original
  method-only spec fixture remains an acceptance fixture; a new field fixture
  preserves the rejection intent, and the imported negative uses that shape.
- `scripts/check-snippets.json`: kept main's current registrations and its
  removal of obsolete registrations. This lane adds no registration there.
- `scripts/qa/corpus.py`: kept main's `diagnostic_codes_match` helper and
  phase-aware `expected_cycle` guard, which supersede the lane's earlier
  inline E0505 guard without losing the separate required diagnostic check.

The solver/inference changes merged automatically. Main's whole-service
inhabitance check, no universal-entry removal, explicit duplicate-provider
E0225 behavior, and checker performance changes are retained. No changes
relative to main remain in `std/`, `lib/`, or `examples/`: all 75 exported
Fail conversions in `effects-e4.md` and their adapted callers are unchanged.
The first merged focused `deferred` row-scheme run passes both tests,
including method-only acceptance and required-field rejection locally and
across modules. Final merged checking is recorded separately below.


The merged checker gate passes **363 tests**, with four existing stress tests
ignored and only `spec_corpus_reverse_order_and_recovery` filtered out.
All 53 row-scheme tests and all current main goldens retain their error lines.
Strict all-target clippy on `ilang-check` and the release CLI build pass.
A subsequent focused `deferred` run passes both tests after adding the
required-value-field lower-bound control in both call orders, while retaining
its method-only counterpart. Chapter 4.4 passes all five cases at O0/O2.
