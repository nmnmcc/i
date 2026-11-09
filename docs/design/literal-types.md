# Design: literal types for Int and String

Status: draft for review (S2). Base commit: 1865366. Line numbers refer to that commit.

This note is the design for track T1. It starts from the "soundness first" proposal and adds what the prototype on `spike/s2-proto` (bbe0726, never merged) found. Examples use the new syntax of `docs/design/syntax.md`: matches are `e |> { … }`, guards are `p ? c -> e`, and type variables are `UpName`s. The plan has A2 convert `docs/spec.md` before T1 starts, so the spec text below is written for the converted spec.

## 1. What the programmer sees

```i
method = "GET"                            // method : "GET"
codes = [200, 404]                        // codes : List(200 | 404)
kind = { "a" -> 1  "b" -> 2 }             // kind : ("a" | "b") -> 1 | 2
size = (s: String) { s |> { "" -> 0  rest -> string.length(rest) } }
                                          // rest : String - ""
send = (m: "GET" | "POST") { m |> { "GET" -> 1  "POST" -> 2 } }
                                          // covered, no `_` needed
"a" == "b"                                // E0520: never equal
(s: String) { s == "a" }                  // correct
1 + 2                                     // Int, not 3
-1                                        // -1, and so is -{1}
-x                                        // Int
1.5                                       // Float; Float has no literal types
```

## 2. The problem

Today `"a"` has the type `String`, and `1` has the type `Int`. So:

- A `case` on a closed set of strings or numbers, such as HTTP methods, log levels or status codes, is never covered without `_`. The type cannot say "only these values".
- In `s |> { "a" -> …  rest -> … }`, `rest` is still `String`. The type cannot say "every string but `"a"`".
- `"a" == "b"` is not found as a comparison that is always `False`.
- Structural records (S3) need their keys as types: `record.get(r, "name")` must know which field `"name"` is.

These four uses are unrelated, and all of them follow from one idea: a type is a set of values (§4.1), so one value is a type too. That is a literal type, as in TypeScript.

## 3. Decisions

Each decision has its reason.

1. **A member has a value set.** For `Int` and `String`, a member of a type is a type constructor plus a value set. A value set is one of: all the values; a finite set (`"a" | "b"`); or all the values but a finite set (`String - "a"`). Every other type constructor always has all its values.
   *Reason:* finite and cofinite subsets of an infinite set are closed under union, intersection and complement. So each head is still a Boolean algebra, the type lattice stays distributive, and the rules of MLsub and MLstruct still give principal types.

2. **The set is a field of `Ty::Con`, not part of `Head`.** `Ty::Con(Head, ValueSet, Vec<Ty>)`.
   *Reason:* `Head` is `Copy + Hash` and a key in `type_tests`, in coverage and in `Place.seen`. More important, `"a"` and `"b"` must stay one member with one head, so that `union_in`, `route` and coverage keep the rule "one member per head" (§4.4). With the set in `Head`, `"a"` and `String` would be different members: `union_in` would not merge them, `route` would pick only one, and `known::disjoint` would call them disjoint (a false E0520). A separate `Ty::Lit` variant was rejected for the same reason: it is a second kind of member for the `Int` and `String` heads, and every rule that finds "the member with this head" would have to look for it too.
   *What the compiler does not find:* the new field makes the compiler stop where a `Con` is built or bound with its parts named, but not at the 8 sites that match `Ty::Con(..)` (solve.rs :298, :397, :442, :552 `diff_left`, :779 `inter_in`, :893 `diff_in`; display.rs :870 `Norm` diff; known.rs :333 `match_k`), and not where two members are compared with `.head()` (solve.rs :320, :519–521, :711, :782; known.rs :27, :69, :357; coverage.rs :231; pattern.rs :474; display.rs :711, :739, :871). These sites are where most set rules belong. Step 3 of §12 lists them from a search for `Ty::Con(..)` and `.head()`, and each one is changed or marked "the set does not matter here" in the same commit.

3. **Only `Int` and `String` carry sets, and they have no arguments.** So a set never meets variance, type arguments, env mode or `compare()`.
   *Reason:* this keeps every new rule small. Float has no literal types, because Float equality is not identity (`0.0 == -0.0`). BitArray and UtfCodepoint have none, because no use needs them.

4. **No widening, no template literal types, no literal arithmetic.** `x = "a"` gives `x : "a"`. `1 + 1 : Int`, `"a" <> "b" : String`, `-x : Int`. An interpolated string, `"user-\(n)"`, has the type `String`: it is not a literal, so it has no literal type, and it may not be written as a type or as a pattern.
   *Reason:* values are never changed, so TypeScript's reason for widening (a `let` that is assigned later) does not exist. Widening would also be a special rule that breaks principal types. Template literal types (`"a" <> String`) would make the algebra much bigger, for little use.

5. **`-` on an integer literal gives a literal type.** When the operand of the unary `-`, with its groups taken away, is an integer literal, the result has the literal type of the negative number: `-1 : -1`, and `-{1} : -1` too. Every other `-` gives `Int`: `-x`, `-{-1}`, `-f()`. (`-(1)` is a compile error, because `(1)` is a tuple of one part.)
   *Reason:* patterns already read `-1` this way (§7.1), and direct values already count "a literal, also with `-` before it" (§6.10). It would be odd if `-1` could be written as a pattern and a type, but not get its own type as an expression. The rule looks through groups because `meta.md` §1 says the core trees `(- 1)` and `(- {1})` are the same code, so they must get the same type; there is no negative literal atom in the core tree.

6. **A literal pattern gets its literal type only in a match.** In a clause block with two or more clauses, or with a guard, a literal pattern has its literal type (rule 11). Everywhere a pattern is *fitted*, it has the whole `Int` or `String`: in a binding, a parameter, a clause block with one clause and no guard, and a value inside a bit-array segment. It also has the whole type at a list item place, or below one, because all the items of a list share one element type (§5.3).
   *Reason:* fitting needs `B <: Q`. With `Q = "a"`, `"a" = s` with `s: String` would become a type error, but today it compiles and may fail through `Assert`. The prototype first gave literal types in bindings too, and that made 10 false E0501 in std (`0 = string.byte_size("")`, `Error(1) = list.try_each(…)`) and 2 in spec. A segment value such as `<<"hi": utf8, …>>` is matched against a decoded value of the whole segment type, so it must have that type, or `String <: "hi"` fails.

7. **Known types must hold every possible value.** Two changes:
   - In matching (§4.8), a type variable matched with a `_` part, or with a generalized type variable, gets `_`; and a variable member of a union pattern gets `K` less the other members *by value*, not by head.
   - **Output places of a function parameter.** Let `P` be the type of a parameter of the called function, where `P` has a function type in it. An *output place* of `P` is a place under a function type in `P` where the argument gives values to the called function: the result and the errors of that function type, a parameter of one of its parameters (a continuation or a setter it is given), the arguments of the services in its environment, and the same places again inside them. A type variable that is in an output place of some parameter type gets its known type made whole by `whole_lits` (below). This is done in **both** places where known types of a call are matched: in the expected type of a function expression given as an argument (§4.8, "come last"), and in `call_known`, the known type of the call's value. It is done for every argument whose parameter type has such a place, not only for function expressions, because a named function given as the argument may give values through the same places.
   - **`whole_lits(t)`** makes a known type whole in its parts, at every depth. An `Int` or `String` member at a covariant place (the type itself, an argument of a tuple, a covariant argument of a constructor, the result or errors of a function type) gets all its values. A part at any other place (a parameter of a function type, an invariant or bivariant argument of a constructor, an environment) becomes `_`, because there a whole `Int` would make the set of values smaller, not larger. So `0` becomes `Int`, `(0, 0)` becomes `(Int, Int)`, `Box(0)` becomes `Box(Int)`, and `(0) -> 1` becomes `(_) -> Int`.
   *Reason:* E0520 is only right when a known type is never smaller than the set of values that can come, and §4.8 rule 13 adds the constraint that a value is a subtype of its known type, so a known type that is too small also gives a false E0501. Before S2, `result.unwrap(Ok(1), 50)` had the known type `50`, because `Ok(1)` matched nothing. With `Int` for both, nobody saw it. With literal types it gives a false E0520 on `result.unwrap(Ok(1), 50) == 1`, which is `True` when run. The prototype's fix removed the 3 false errors and changed no correct E0520. The second change is for values that come back from a function argument, which the other arguments do not show:
   - the result: in `list.fold(xs, 0, (n, x) { n == 3 |> { … } })`, `acc` is matched with `0` only, so `n` would get the known type `0`, and `n == 3` a false E0520, though `n` is 3 on the fourth element. The same holds for `index_fold`, `try_fold`, `map_fold` and every loop helper written by users;
   - a parameter of a parameter: in `loop = (init: S, body: (S, (S) -> Nil) -> Nil) -> Nil { … }`, the call `loop(0, (n, again) { … again(n + 1) … })` gives `S` the value `n + 1` through `again`, and `capture(0, (k) { k(5) }) == 5`, with `capture: (x: A, f: ((A) -> Nil) -> Nil) -> A`, is `True` when run;
   - a service argument: `step: (S) -> Bool ! State(S)` where `step` calls `set(n + 1)`.
   Making the parts whole, and not giving `_` for the whole variable, keeps `s.count` correct in `list.fold(xs, State(0), (s, x) { … s.count … })` and `.0` correct in `fold(xs, (0, 0), (acc, x) { … acc.0 … })`, where the field needs a known type. If `whole_lits` stopped at the top member, `(0, 0)` would stay as it is, and rule 13 would give the false error "`Int` is not a subtype of `0`" at `(0, 0)`. Variables only in input places keep their literal types: in `list.map(["a"], (s) { s == "b" })`, `A` is a parameter of `(A) -> B`, so `s` keeps `"a"` and the E0520 is right. With a callback that gives back a value of another type constructor than the start value, the known type is still too small; that is so at 1865366 too (`fold(xs, Nil, …)` with a callback that gives `Int`), so it is not S2's, and it gets its own ticket.

8. **E0520 compares value sets.** Two members are disjoint when their heads differ, or when they are both `Int` or both `String` and their sets have no value in common. So `x = "a"` then `x == "b"` is an error, as in TypeScript.
   *Reason:* §6.9 already says "disjoint by §4.5 rule 3"; rule 3 just learns about value sets. No new rule is added.

9. **The host may only *give* whole `Int` and `String`.** Where i code gives a value to the host (parameters of an external function, values and errors of a host export, parameters of a host-service function), an `Int` or `String` member counts as its type constructor, whatever its value set: a literal type, a union of literal types of one type constructor (`"info" | "warn"` counts as `String`), and all the values but a finite set (`Int - 0`, which narrowing gives in `(n: Int) { n |> { 0 -> 1  m -> m } }`). Where the host gives a value to i code (the value of an external function, the parameters of a host export, the values and errors of a host-service function), an `Int` or `String` member must have all its values. The existing errors E0318, E0320 and E0521 report it; no new code.
   *Reason:* the host's values are checked only for their form (§B.2), not against a value set. Without this rule, `@external … -> "a" | "b"` or an export whose parameter is inferred as `"a" | "b"` lets the host give `"c"`. This is an extension of §10.3's existing list, not a new kind of rule. A run-time equality check at the boundary is the other choice; it is left as an open question (§13), because it changes C's code in `emit/host.rs`.

10. **`type_tests` carries the sets.** `HashMap<(ModuleId, Span), Vec<TestMember>>`, with `TestMember { head: Head, values: TestValues }`. A set other than "all" becomes an equality test in `lower.rs`, with the `Cond`s that exist. No new `Cond`.
    *Reason:* the test of `x: "a" | "b"` must look at the value, and codegen has no other way to know the set.

11. **Long literal unions are shortened only in text for people.** More than 12 literals of one head in a message or a hover print the first 10, then `… (N more)`. `Ty`, `ShownType`, `Typed::schemes` and the package interface are never shortened.
    *Reason:* `…` is not a token, so a shortened type can never be read back as another type, and package identity stays exact.

12. **Int values are kept as canonical decimal text.** This is what `coverage::Lit::Int(String)` already does (coverage.rs:30): `0x10` is `16`, and `-0` is `0`. Strings are kept as their decoded scalar values, so `"\u{e9}"` and `"é"` are one value (§11.4). Display sorts Ints by number; the sets themselves only need some fixed order, and byte order is used.
    *Reason:* one canonical form shared by the checker and coverage, so the two can never disagree, and no BigInt work in the hot path.

13. **A literal type is a written type in both surfaces of `meta.md`.** The Lisp surface gets a type row for literals: `"a"`, `1`, and `(- 1)` for `-1`. `members` gives one item for each value of a finite literal member. In the position of a type, the arguments of a macro call are just types, because integer literals and strings are now types. (§5.9.)
    *Reason:* `meta.md` promises that every `.i` file can be changed into a `.is` file that is the same code (§1), and that a macro gets and gives types as the `Syntax` of written types (§7.4). Without a row, `send = (m: "GET" | "POST") { … }` has no `.is` form, and `type_of(x)` for `to_text(1)` (meta §7.1) gives a type that `subtype` calls a `SyntaxError`. A macro that builds a switch from `members(type_of(m))` with `m: "GET" | "POST"` must see two values, not one member it cannot take apart.

## 4. Representation

New file `crates/ilang-check/src/types/lits.rs`. It is pure: no solver access.

```rust
/// The values of an Int or String member. `ValueSet::ALL` is `None`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Default)]
pub(crate) struct ValueSet(Option<Lits>);      // 8 bytes, through the niche

#[derive(Clone, Debug)]
pub(crate) struct Lits(Arc<LitsInner>);
#[derive(Debug)]
struct LitsInner {
    hash: u64,              // computed once
    except: bool,           // false: these values; true: all values but these
    items: Box<[Box<str>]>, // sorted by bytes, no duplicates, never empty
}
impl PartialEq for Lits {   // O(1) in the common case
    fn eq(&self, o: &Self) -> bool {
        Arc::ptr_eq(&self.0, &o.0)
            || (self.0.hash == o.0.hash && self.0.except == o.0.except && self.0.items == o.0.items)
    }
}
impl Eq for Lits {}
impl Hash for Lits { fn hash<H: Hasher>(&self, h: &mut H) { h.write_u64(self.0.hash) } }
```

- Canonical form, enforced by the only constructors `ValueSet::only(items) -> Option<ValueSet>` and `ValueSet::except(items) -> ValueSet`: a finite set is never empty (the caller gets `None`, which means `Never`), and "all but nothing" is `ALL`. So equal sets are equal values with equal hashes, and the solver cache (solve.rs:94, :100) and the `l == r` shortcut (:246) stay right.
- `Ty::lit(b: BuiltinType, s: ValueSet) -> Ty` is the only way to make a member with a set other than `ALL`. It returns `Ty::Never` for an empty set. In debug builds it asserts that `b` is `Int` or `String`. `Ty::named` and `Ty::tuple` give `ALL`.
- **No helper rebuilds a `Con` with a default set.** Every rebuild site binds the set and passes it on: `Ty::Con(h, s, args) => Ty::Con(*h, s.clone(), new_args)`. Only the places that make a new whole head (`named`, `tuple`, the fresh heads in `narrow` and in type tests) write `ALL`.
- `Ty::values(&self) -> &ValueSet` gives `ALL` for every type that is not a `Con`.
- Add `const _: () = assert!(size_of::<Ty>() <= N);` with `N` the size at 1865366 plus 8.
- The public form for codegen and tests: `pub enum TestValues { All, Only(Vec<LitValue>), Except(Vec<LitValue>) }`, `pub enum LitValue { Int(BigInt), Str(String) }`.

### The algebra

Over one infinite domain D (all Ints, or all Strings). `F` is "only these", `E` is "all but these".

| Op | Rule |
|---|---|
| union | All ∪ x = All. F(A) ∪ F(B) = F(A∪B). F(A) ∪ E(B) = E(B∖A). E(A) ∪ E(B) = E(A∩B). |
| meet | All ∩ x = x. F(A) ∩ F(B) = F(A∩B). F(A) ∩ E(B) = F(A∖B). E(A) ∩ E(B) = E(A∪B). |
| complement | All → empty. F(A) → E(A). E(A) → F(A). |
| minus | x ∖ y = x ∩ complement(y). |
| subset | x ⊆ y iff x ∖ y is empty. So E ⊆ F is never true (D is infinite), F(A) ⊆ E(B) iff A∩B = ∅, E(A) ⊆ E(B) iff B ⊆ A. |
| disjoint | x ∩ y is empty. Only possible when one side is F. |
| contains | v ∈ x. |

`meet` and `minus` return `Option<ValueSet>`, where `None` is empty. Each op is one sorted merge, O(n+m). There is also `join_all(&[ValueSet])`, a k-way merge, used where many literals are joined at once (§9).

## 5. Exact rules

`S(t)` is the value set of a `Con`. "Same head" means `h1 == h2`, as today; sets only matter when the heads are the same.

### 5.1 Solver (solve.rs)

1. **`(Con h S args, Con h T args')`**, `constrain_in` (:245, the Con arm near :326). If not `S ⊆ T`, fail with Mismatch, before the arguments. Then the arguments, as today. In env mode both sets are `ALL` in practice; require `S == T`.
2. **`(l, Diff(a, H))`** (near :320). Fail with Mismatch when some `m ∈ H` has the head of `l` and `S(l) ∩ S(m)` is not empty. Otherwise `l <: a`. For heads with all values this is today's rule, since All ∩ All is not empty.
3. **`route(l, ms)`** (:516). Let `m` be the member of `ms` with the head of `l` and set `X` (by §4.4 there is at most one; take the union of all of them if a union did not come through `union_in`).
   - If `S(l) ⊆ X`: `l <: m`, as today.
   - If not, split. The part `S(l) ∩ X` needs nothing (no arguments). The rest `h(S(l) ∖ X)` goes to the one variable-like member (a variable, `Diff` or `Inter`), as in today's path with no same-head member: one such member is a constraint, none is Mismatch (shown with the whole `l`), more than one is Ambiguous.
   - This is principal: with `m` free of variables, `l <: m | v` holds iff `l ∖ m <: v`. There is no choice.
4. **`diff_left(a, H, r)`** (:544), for `A - H <: R` and a member `a = h(S)` with `m ∈ H` of the same head and set `X`. Three cases:
   - `S ⊆ X`: `a <: m`, as today (for a head with arguments).
   - `S` and `X` disjoint: `a <: r`.
   - Both: `h(S ∖ X) <: r`. The part in `X` needs nothing.
   With no `m`: `a <: r`, as today.
5. **`union_in`** (:698) and **`merge`** (:801). Same head: the merged `Con` gets `S ∪ T`. Before merging, join all literal members of each argument-less head with one `join_all`, so `k` literals cost one k-way merge, not `k` merges. In env mode, keep two members apart unless their sets are equal.
6. **`inter_in`** (:744). Same head: `S ∩ T` first; if empty, return `Ty::Never` before the arguments are merged. The `{A | B} & C` distribution is unchanged.
7. **`diff_in`** (:877), `a = h(S)` not a variable, `m ∈ H` of the same head with set `X`:
   - a head with arguments: `a <: m` and `Never`, as today;
   - `Int` or `String`: return `Ty::lit(h, S ∖ X)`, which is `Never` when empty. No constraint.
   - This is the main change in meaning: `String - "a"` is no longer `Never`; it is every string but `"a"`.
   - For a variable `a`: `Diff(a, H)` and `diff_bound` (`a <: H | r`), as today. Rule 3 makes this right when `a` later gets `String` as a lower bound: `"a"` stays in `H` and the rest goes to `r`.
8. **`add_bound`** (:385), only for `Con`s with no arguments:
   - Lower bound `l` with set `S`, and a stored lower bound of the same head with set `T`: if `S ⊆ T`, skip (every upper bound was already checked against the larger bound). Otherwise propagate `l` as today, then replace the stored bound with `S ∪ T`; do not push a new one.
   - Upper bound `u` with set `S`, and a stored upper bound with set `T`: if `T ⊆ S`, skip. Otherwise propagate as today, then replace the stored bound with `S ∩ T`, unless that is empty: then keep both, because `reach()` skips `Never` and the bound would be lost.
   - So a parameter called with 1000 different constants keeps one bound per head, not 1000. The prototype lacked this: `bounds.contains` no longer finds duplicates once each literal is its own type.
9. **`compare`** (:452) gets no set check. The upper-bound dedup in `add_bound` ("check a new bound only with the first of its head") is right only because `compare` looks at arguments and arity alone.
10. `map_children`, `level_of`, `bound_vars`, `env_vars`, `needs_join`, `extrude`, `reach`: pass the set through and ignore it; sets have no variables.
11. **A failure only of values.** A new `FailKind::Values` is used for a failure that comes only from value sets: rule 1 when the heads and arguments agree but `S ⊄ T`; rule 2 when the head of `l` is in `H` only with sets that meet `S(l)`; and rule 3 when `ms` has a member with the head of `l` and the rest `h(S(l) ∖ X)` has no variable-like member to go to. It is a `Mismatch` in every way but its kind, and it is reported as a `Mismatch` (E0501) everywhere except in rule 11 of a match (§5.3). The test is made where the failure is found, not later from the two types, so it works when the right side is a union (`Int <: 0 | Nil`).

**Why this is sound.** Each rule is the set meaning of §4.1 for one head. The only new splits (rules 3, 4 and 7) are equivalences, not approximations: for a head with no arguments, `h(S) <: h(X) | V` iff `h(S ∖ X) <: V`. No rule gives a lower bound more values than flow to it, and no rule takes values out of an upper bound without a constraint.

### 5.2 Literal expressions and written types (infer.rs)

- `ExprKind::Int(n)` (:1076) has the type and known type `Ty::lit(Int, only {n})`. `ExprKind::String(s)` (:1078) has `only {s}`.
- **Interpolation.** A string with interpolation, `"user-\(n)"` (the plan's syntax, added by A1 as a new expression kind or as parts of a string literal), has the type and known type `String`, whole. Only a string literal with no interpolation has a literal type. In a type, and in a pattern, a string with interpolation is a compile error ("a string with `\(…)` is not a literal"), because it has no fixed value. If A1 keeps interpolation inside `ExprKind::String`, the parts decide: one text part and nothing else is a literal. `id = (n: Int) { "user-\(n)" }` has the type `(n: Int) -> String`.
- `Unary { Neg, e }` (:1189), where `e` with its groups taken away (a `Block` with one `Stmt::Expr` and nothing else, as many times as it is there) is `Int(n)`, has `only {-n}`, with `-0` as `0`. Any other `Neg` gives `Int`. One helper, `neg_literal(e) -> Option<&str>`, is used here, in `written_known` and in `known.rs`, so the three can not disagree.
- `known.rs` `written_known` (:550–560), the known type of a name used in the value of its own group, gives the same literal types: `ExprKind::Int` and `ExprKind::String` give their literal types, and `Unary Neg` gives the literal type when `neg_literal` finds one, else `Int`. Without this, `pair = (1, f)` and `f = () { pair.0 == 2 }` in one group gives no E0520, while the same test outside the group does.
- The operators of §6.9 give whole types, except `-` on an integer literal, also inside groups (above). Their operand constraints (`<: Int`, `<: String`) take literals unchanged.
- A written literal type becomes `Ty::lit` in `conv` (:852), through a new `hir::TypeKind::Lit` made by `resolve.rs` `fn ty` (:1329). A float literal in a type is a syntax error with the message "Float has no literal types".

### 5.3 Patterns (pattern.rs)

- **`leaf`** (:213), which already takes `in_case`:
  - `in_case` (a clause block with two or more clauses, or with a guard): `Int` and `String` literals give their literal types;
  - fitting (a binding, a parameter, one clause with no guard): the whole `Int` or `String`;
  - a value inside a bit-array segment: the whole type of its segment, in every case;
  - `"lit" <> r`: `String`, and `r` gets `String` (no template literal types);
  - Float and bit-array patterns: unchanged.
- **Rule 11** keeps its shape. `P` now holds literal types:
  - no catch-all: `B <: "a" | "b"`, so an unannotated subject infers `"a" | "b"` (as TypeScript does);
  - a catch-all: `B <: P | r - P`, and the route split sends the rest of `String` to `r`.
  - **List items keep the whole type.** A literal at a list item place (`Step::Item(i)`), or at any place below one, has the whole `Int` or `String` in `P`, as where it is fitted. All the item places of a list share one element type variable (pattern.rs `visit` makes every `Step::Item(i)` child with `e.clone()`), so a literal at one item place would hold every item to it. In `(args: List(String)) { args |> { ["git", "push"] -> 1  [] -> 2  [_, ..] -> 3 } }`, item place 1 has no catch-all, so a literal type there would give `String <: "push"` for every item, and a false error on a match that is covered. Coverage still uses the value sets of the subject, so `(xs: List("a" | "b")) { xs |> { [] -> 0  ["a", ..] -> 1  ["b", ..] -> 2 } }` is covered.
  - **The error when values are not taken.** With a subject whose type is known when the block is checked, `(n: Int) { n |> { 0 -> "zero"  1 -> "one" } }` now fails in rule 11 (`Int <: 0 | 1`), before coverage runs; so do `Ok(0)` and `Error(_)` on `Result(Int, Nil)` (`Int <: 0` under `Ok`), and `(n: Int | Nil) { n |> { 0 -> "zero"  Nil -> "none" } }` (`Int <: 0 | Nil`). Today these are E0516 with a witness, and they must stay so. The `self.sub` of rule 11 (pattern.rs :460) becomes `self.sub_case`, which flushes like `sub` but **holds back** a failure of kind `FailKind::Values` (§5.1 rule 11) and reports every other failure as today. After `check_case` runs coverage for the same match, a held-back failure is dropped when coverage gave an E0516, and is reported as E0501, as today, when it did not; so no failure is ever lost, even where coverage cannot see the values (a subject with a type variable in it). In the cases above coverage always reports: the values that rule 11 could not take are values of the subject's type that no literal clause matches, so `check_case` finds them in coverage and gives one E0516, at the subject's span, with its own witness (`2`, `Ok(1)`, `1`). `sub_case` makes no E0516 itself, so the user never gets two. A failure that comes later through a type variable (an unannotated subject, then `kind("c")`) is an ordinary E0501 at the call, as for any argument.
- **Rule 12 (`narrow`, :563).**
  - Rule 12 works at every place, also below constructors and inside tuples. A candidate there is taken only when the whole row is matched by earlier clauses, which the `useful` check below decides with every other place of the row: `(p: (String, Bool)) { p |> { ("a", True) -> 0  (t, _) -> g(t) } }` does not narrow `t`, because `("a", False)` comes to the second clause.
  - The candidates at a place are: the whole heads seen in earlier unguarded clauses (as today), each `Int` or `String` literal seen there (`Lit::Int` and `Lit::Str` only; a Float literal, a `"p" <> r` prefix and a bit-array pattern are never candidates, because they have no literal type), and each member of an earlier type test (its head with its set).
  - A candidate is taken when the catch-all, replaced by a pattern that matches exactly that candidate, is not useful against the earlier rows. That pattern is `Pat::Head` for a head, `Pat::Ctor(Ctor::Lit(v))` for a literal, `Pat::Or` of `Ctor::Lit` for a finite type-test member, and `Pat::Except` for a cofinite type-test member. How `useful` takes a `Pat::Except` in the first place of `q` is in §5.5.
  - **Fast path:** a literal that is the whole pattern of an earlier unguarded clause at that place, with catch-alls at every other place, is taken without calling `useful()`.
  - `H` is the union of the taken members; taken literals of one head join into one member. The catch-all gets `B - H`.
  - The known-type filter (:474) becomes `display::known_diff(k, H)`, not a filter by head.
- **Type tests `x: T`** (:723–790, the insert at :757). Each member gives a `TestMember`. In coverage: a member with all values is `Pat::Head`; a finite one is `Pat::Or` of `Ctor::Lit`; a cofinite one is the new `Pat::Except(head, X)`. Rule 11 adds `B <: T | r - T` as today; with the new difference, `r` gets exactly the values not in `T`. `CASE_TYPE_VAR` (E0511) is unchanged.
- **`may_fail`** (near :344) needs no change: with sets in coverage, `"a" = x` with `x: "a"` can not fail, and codegen already reads `may_fail`.
- **W0501 message.** When the subject of a match has a type with one value, add: "the value matched is always `1`". The prototype's 5 new W0501 were all this case.

### 5.4 Known types and E0520 (known.rs)

- `members` (:22): a `Diff` uses `display::known_diff` (set difference); an `Inter` of headed members uses `display::known_inter` (set meet).
- `member` and `one_member` (:40, :47): unchanged, by head.
- `disjoint` (:64): heads differ, or the same head and `lits::disjoint(S, T)`. A member that is `_` or a type variable is never disjoint.
- `member_for` (:434): an `Int` or `String` literal pattern gives `one_member(b, head)` met with `only {v}`; `Prefix` gives the member unchanged.
- `written_known` (:550): literal types, as in §5.2.
- `match_k` (:322), the first fix of decision 7:
  - when the part of `K` is `_` or a generalized type variable, every type variable in the part of `P` there gets `_` (it pushes `Ty::Top`; today the early return at :323–330 pushes nothing). So in `call_known`, `acc` of `list.fold(xs, 0, f)` gets `0 | _`, which is `_`, when the known type of `f`'s result is `_`;
  - in the `Union` branch, a variable member of the pattern gets `K` less the other members by value set, not by head.
- **The second fix of decision 7.** A new `known::output_vars(p: &Ty) -> Vec<TyVarId>` gives the type variables in the output places of a parameter type `p`: it walks `p` with a polarity that starts as "given by the argument" (the argument is `p`'s value) and flips at each function parameter. A variable is an output variable when it is at a place given by the argument and below at least one function type: the result and errors of a function type given by the argument, a parameter of a parameter, and so on at any depth. A service argument in an environment, and an invariant or bivariant argument of a constructor, counts as both. A variable at the top of `p`, or only under tuples and constructors, is not: the argument's own known type shows those values.
  - infer.rs `call` (:1308–1321): before the pass with `later = true`, for every `i`, replace the entries in `sigma` of each variable in `output_vars(ps[i])` with `known::whole_lits(t)`.
  - known.rs `call_known` (:413): after matching all the arguments, do the same for each variable in `output_vars` of every parameter type, before `ksubst` of the result.
  - `whole_lits(t)` (known.rs, new): every `Int` and `String` member at a covariant place, at any depth (tuple arguments, covariant constructor arguments, results and errors of function types), gets all its values; a part at a contravariant, invariant or bivariant place, or in an environment, becomes `Ty::Top`. Variances come from `self.s.variances`, as in `ctor_known` (:208).
  - So in `list.fold(xs, 0, (n, x) { … })`, `n` has the known type `Int`, and with `(0, 0)` as the start, `(Int, Int)`. In `loop(0, (n, again) { … })`, with `body: (S, (S) -> Nil) -> Nil`, `S` is the parameter of a parameter, so `n` is `Int`. `capture(0, (k) { k(5) })` has the known type `Int`. In `list.map(["a"], (s) { … })`, `s` keeps `"a"`, because `A` is a parameter of `(A) -> B`, an input place.
- `ksubst_in` (:382): passes sets through.
- **E0520 message** (infer.rs near :1244, which handles `==` and `!=` together). The result that is always the same is `False` for `==` and `True` for `!=`. When both known types are one value each, say: "both sides are constants, `1` and `2`, so `==` is always `False` (§6.9). To compare two constants when the program runs, pass them as parameters." When they are finite sets but not both one value: "the values `"a" | "b"` and `"c"` are never equal, so `==` is always `False` (§6.9)". In every other case the text of today. Types in the message use the shortened text.

### 5.5 Coverage (coverage.rs)

- `Vals.heads` entries become `(Head, ValueSet, Vec<Ty>, Mode)`. `values()` (:173) reads the set from the `Con`. Union and inter of `Vals` (:131) use the algebra, and drop an entry whose set is empty. The `Diff` case (near :229) subtracts by set; only a subtraction with all values removes a whole head.
- **A side with `other`** (a part such as `r - 0` where `r` may be any value) counts as all values for every head. `values()` cannot write `r - 0`, and without this rule `inter` lost the `Int` values that come through that part, which gave false "never used" warnings in `sign`.
- `signature(h, col)` (:518): a finite set gives `Some(the Ctor::Lit list)`. A cofinite set or all values gives `None`, as today.
- Specialization by `Ctor::Lit(c)` keeps: `Lit` rows equal to `c`; `Prefix` rows that `covers(c)` (:267); `Except(h, X)` rows with `c ∉ X`; and `Head`, wildcard and `Or` rows as today. The missing-constructor search in `useful_head` (:420) uses the same test, not `==`, so `"a" <> _` covers `"ab"` in a finite signature.
- **`useful_head` for `Int` and `String`** (:420–440 and the finite branch after it). Today a head with no finite signature checks only the default matrix. That is right only because every default row is also in every specialization, and an `Except(h, X)` row breaks it: it is in the default matrix but not in the specialization by a value in `X`. So for `Int` and `String` the two branches are replaced by one rule:
  1. Let `L` be the **listed values** of the column for `h`: the values of the `Lit::Int` and `Lit::Str` rows, the `X` of every `Except(h, X)` row, and, when the signature is finite, every value in it. Take out the values that are not in the column's set.
  2. For each `c` in `L`, specialize by `Ctor::Lit(c)` (with the row filter above, so an `Except(h, X)` row stays only when `c ∉ X`) and check `rest_q` against it. The first that is useful gives the witness `c`.
  3. When the column's set is not finite (all values, or all but a finite set), also check the default matrix: the `Wild` and `Head` rows, the `""` prefix rows, and every `Except` row. Each of these matches every value not in `L`, because every `X` is in `L`. A useful default gives a witness from the column's set less `L` that no non-empty prefix row of the column `covers` (the witness rule below). A non-empty prefix row is not in the default, as today, which may only make the answer "useful"; the witness must still be a value that such a row does not match, or the message names a value that a clause matches (`s |> { "" -> 0  "a" <> _ -> 1 }` must not say `"a"`).
  4. When the signature is finite, `L` holds all of it, so no default is checked.
  This is what makes `s |> { "a" -> …  x: String - "a" -> … }` covered, `s |> { "a" -> 0  x: String - "a" - "b" -> 1 }` not covered with the witness `"b"`, `(m: "a" | "b" | "c") { m |> { "a" -> 0  x: String - "a" -> 1 } }` covered, and `(s, b) |> { (x: String - "a", _) -> 1  ("a", True) -> 2 }` not covered with the witness `("a", False)`. The cost is one specialization per listed value, as for a finite signature today. Other heads are unchanged.
- **`Except` in `q`.** Narrowing (§5.3) may put `Pat::Except(h, X)` first in `q`. `useful` takes it as `useful_head` with step 2 skipping the values in `X` (they are not in `q`) and step 3 as written. `useful` today matches `q[0]` against `Or`, `Ctor`, `Head` and `Wild` only, so this is a new arm.
- The `Except` row and step 3 have no evidence from the prototype, so they get their own tests (§10), at O0 and O2.
- `sub_cols` (:482): a `Ctor::Lit(c)` whose value is not in the column's set for that head is not useful. So a `"a"` clause on `String - "a"` is reported as never used.
- **Witnesses.** For a finite column, the missing literal. For a column that is not finite, a concrete value not in the listed values `L` of `useful_head` (for a column `E(X)` with no rows, `L` is `X`): for `Int`, the smallest `n ≥ 0` not in `L`. For `String`, let `P` be the non-empty prefixes of the column's prefix rows; the witness is `""` if it is not in `L`, else the first of `c`, `cc`, `ccc`, … not in `L`, where `c` is the first character of `a`, …, `z`, `0`, …, `9` that no prefix in `P` starts with (there are more of these than prefixes in any real match; if all 36 are used, take the first scalar value above `z` that is not). At most `|L| + 1` tries after `c` is found. For `s |> { "" -> 0  "a" <> _ -> 1 }` this gives `"b"`.
- **Witness text.** Witnesses print literals with the escapes of §1.6 (`\n \r \t \f \" \\ \u{…}`), through the same function that prints `ShownType::Lit` (§5.7), so a witness can be pasted back as a pattern and a type and a witness in one message agree. coverage.rs :586 (`format!("{s:?}")` for `Lit::Str` and `Lit::Prefix`) is changed to use it; Rust's escapes such as `\0` are not i syntax.
- **Direction.** Where something is not exact, coverage must answer "useful" or "not covered", never the other way.

### 5.6 Codegen (crates/ilang-wasm/src/lower.rs)

At the type test (:2128), for each `TestMember`:

```
All          => head_cond(v, head)                                   // unchanged, :2211
Only(X)      => Or(IsInt(v, x) | IsString(v, x) for x in X)          // each IsX checks its own tag
Except(X)    => And(head_cond(v, head), Not(Or(IsX(v, x) for x in X)))
```

- `Not(Or(…))` alone is wrong: it would accept an `Int` where the test is for `String`. The `And(head_cond)` is required.
- `head_cond(Int)` must accept both the i31 and the boxed forms. The tests use 5, 2^30, −2^30−1 and 10^30.
- No new `Cond`, so `control.rs` `cond_uses` and `emit` are unchanged.
- Literal patterns and bindings lower as today.
- Known cost: `IsString` makes a new string on every test (`emit/mod.rs` :458). Moving literal strings into globals, or a switch `Cond` for big sets, is a later step in B's code, not T1.

### 5.7 Display (display.rs, shown.rs, ilang-cli)

- `Norm` repeats the solver rules: `union_in` (:698) and `merge` (:889) join sets (with `join_all`), `inter` (:772) gives `Never` when empty, `diff` (:860) is set difference.
- **The Top rule** (near :730, `full` at :753): `_ - H | T…` is Top when, for each member `h` of `H`, the union has a member `u` with the same head, full arguments, and `S(h) ⊆ S(u)`. So `"a" | _ - "a"` and `0 | _ - 0` print as `_`, and every ordinary `s |> { "a" -> …  _ -> … }` still shows `(s) -> …`. `_ - String | "a"` does not become `_`.
- `same_unmarked` (:224) compares sets. The rebuilders (`Coalescer::go`, `both`, `norm`, `replace`, `tops`, `hide`, `not_general`, `ksubst_in`) carry the set.
- `tree` (:1176, :1280) makes a new `ShownType::Lit(String)`, the written form (`"a"` with the escapes of §1.6, `-1`):
  - a finite set is a `Union` of `Lit`s, Ints sorted by number, Strings by bytes (which is scalar order);
  - a cofinite set is `Diff(Named(String), [Lit…])`, written `String - "a" - "b"`. A negative number there is written `Int - -1`, which the §3 grammar reads back;
  - a union with literals and other members prints the literals where their head would be.
- `ShownType` gets `text_limited(12)` for messages (`show`, `show_known`, E0520, E0516) and the LSP hover: more than 12 literals of one head print the first 10, then `… (N more)`; in a difference, `String - "a" - … - "j" - … (N more)`. `text()` (full) is used for `Typed::schemes`, `i check --types` and the package interface.
- `ilang-cli/src/commands/package.rs`: `.head()` (:1165) maps `Lit` to the `Int` or `String` head, and `fn ty` (:1351) prints `hir::TypeKind::Lit`. A change of literals in an API is a change of type, as intended.

### 5.8 Host boundary (resolve.rs, infer.rs)

- `resolve.rs` `host_type` (:1651) gets a direction. For external functions (:1640) the parameters may be literal types, the value may not. For host-service functions (`host_fns`, :1473) the parameters may be literal types, the values and errors may not.
- `resolve.rs` reads the written `hir::Type`, where `"info" | "warn"` is a `TypeKind::Union`, and both `host_type` and `host_part` (:1571) say no to every `Union` today. So both get one more case: after `expand`, a `Union` whose parts, each expanded again, are all `TypeKind::Lit` of one type constructor, or that type constructor itself, and a `Diff` whose first part is `Int` or `String` and whose other parts are literals of it (`String - "a"`), is that type constructor (`HostType::Int` or `HostType::String`) where i code gives the value, and is not taken where the host gives it. This also covers an alias, `Level = "info" | "warn"`, and the parameters of host-service functions. Without it, `log: (level: "info" | "warn") -> Nil` is E0318, and `host_fn` never puts it in `external_fns`.
- `infer.rs` `host_type` / `host_part` (:503, :528) for host exports: a parameter must have all values; in the value and the errors, an `Int` or `String` member with any value set (finite or all but a finite set) is encoded as `Int` or `String`. So `f = (n: Int) { n |> { 0 -> 1  m -> m } }`, whose value is `1 | Int - 0`, which is `Int - 0`, is still a correct host export, as it is at 1865366.
- The messages of E0318, E0320 and E0521 add: "the host gives this value, so it must be all of `String`, not `"a" | "b"`".

### 5.9 Metaprogramming (docs/design/meta.md, track M4)

Literal types must be written, given and taken in both surfaces, as every other type is. These are changes to `meta.md` and to M4's Lisp parser and printer, not to T1's files; T1 writes them down here so M4's RF can check them (the plan asks M4 to check its interaction with T1).

- **§2.3, Types.** A new row: Literal type, normal `"a"`, `1`, `-1`, Lisp `"a"`, `1`, `(- 1)`. A string or integer atom in the position of a type is a literal type. A `(- k)` with exactly one part, where `k` is an integer atom, is a negative literal type; this does not clash with the difference, which takes two or more parts. So `Int - -1` is `(- Int (- 1))`, `"a" | "b"` is `(| "a" "b")`, and `send = (m: "GET" | "POST") { … }` is `(= send (-> ((: m (| "GET" "POST"))) _ …))`. A float atom in a type is a compile error, as in the normal surface.
- **§2.3, Expressions.** No change. `-1` stays `(- 1)`, the unary operator on a literal, and §6.3 (decision 5) gives `(- 1)` and `(- {1})` the same type, so "same code" means the same type.
- **§4.3, macro calls in a type.** "Types, integer literals, and strings" becomes "Types". An integer literal or a string in that position is now a literal type, and its `Syntax` is the same atom as before, so a macro sees no difference; `m(-1)` is now also correct, as `(m (- 1))`.
- **§7.4, `Types`.** `type_of` may give a literal type, such as `Int(1)` or `Text("x")` as the `Syntax`; `subtype` takes it, so `subtype(type_of(value), `Int)` is `Yes` for `to_text(1)` (meta §7.1). `members(t)` gives one item for each value of an `Int` or `String` member with a finite set, and a member with all values but a finite set as one item, `String - "a"`. The order of the items is `meta.md`'s rule, unchanged: the order of their `show` text, for literal items too. So `members` of `2 | 10 | Nil` is `10`, `2`, `Nil`, not the display order of §5.7; one rule for all items keeps a macro's output the same whatever the members are. And `members` of `"GET" | "POST"` is the two `Syntax` `"GET"` and `"POST"`, and a macro can build a switch from it. This is the only place where "member" in `meta.md` is not the member of spec §4.4, and the row says so.
- **Printer.** `i fmt --to=is` and `--to=i` print literal types with the escapes of the surface; a negative literal type is `(- 1)` in the Lisp surface and `-1` in the normal one, so `Int - -1` round-trips.

## 6. Replacement spec text

The text is plain English, as the spec is. Section numbers are kept; nothing is renumbered. Quoted "old" sentences are as in spec.md at 1865366; A2 changes `case` to "clause block" before T1 applies this.

### §3 Type syntax

Change `Prim` and add `Literal`:

```
Prim      = FnType | TupleType | NamedType | Literal | "_" | "{" Type "}" .
Literal   = [ "-" ] int_lit | string_lit .
```

Add this bullet:

> - A `Literal` is a literal type (§4.2): `"a"`, `0`, `-1`. A `-` followed by an integer literal, where a `Prim` begins, is part of the literal. Every other `-` is a difference. So `Int - -1` is `Int` without `-1`, `Int - 1` is `Int` without `1`, and `-1 | 1` is a union of two literal types. A float literal is not a type, because Float has no literal types, and a string with interpolation (`\(…)`) is not a type, because it has no fixed value.

### §4.1 Types are sets of values

Add after the list:

> A **member** of a type (§4.4) is a type constructor, its type arguments, and a **value set**: the values of that type constructor that are in the member. Only `Int` and `String` may have a value set that is not all of their values (§4.2). For every other type constructor, the value set is all of its values.

### §4.2 Built-in types

Add a subsection at the end:

> #### Literal types
>
> Every value of `Int` and of `String` is also a type, the **literal type** with only that value. It is written as the literal: `1`, `-1`, `"a"`. `1 <: Int`, and `"a" <: String`. Two Int literal types are the same when their numbers are equal, so `0x10` is `16` and `-0` is `0`. Two String literal types are the same when their scalar values are equal (§11.4).
>
> The value set (§4.1) of an `Int` or `String` member is one of these:
>
> - all the values: `Int`, `String`;
> - a finite set of values: `"a" | "b"`;
> - all the values but a finite set: `String - "a" - "b"`.
>
> Union, intersection and difference of these sets give a set of the same kind, so the rules of §4.4 and §4.5 always have an answer, and inference still finds principal types (§4.7). `Float`, `BitArray` and `UtfCodepoint` have no literal types.
>
> Values are never changed, so a literal type is never made wider: `x = "a"` gives `x` the type `"a"`, and `[200, 404]` has the type `List(200 | 404)`. Such a value may be used where a `String` or an `Int` is needed, because `"a" <: String`. Operators give the whole type: `1 + 1` has the type `Int` (§6.9).

### §4.4 Union, intersection, and difference

- After "Members of a union with the same type constructor are put together into one." add:
  > For `Int` and `String`, the value set of the member is the union of their value sets: `"a" | "b"` is one member, and `"a" | String` is `String`.
- In the intersection bullet, after "…and equal where it does not change." add:
  > For `Int` and `String`, the value set is the values in both; when there are none, `T & U` is `Never`. So `"a" & String` is `"a"`, and `"a" & "b"` is `Never`.
- Keep "`A - B` takes out of `A` every member whose type constructor is the type constructor of a member of `B`." and add after it:
  > For `Int` and `String`, it takes out only the values of the value set of that member of `B` (below).
- Replace the sub-bullet "When `T` and `U` are not type variables: `T - U` is `Never` when they have the same type constructor, and `T` when they do not. …" with:
  > - When `T` and `U` are not type variables: `T - U` is `T` when they have different type constructors. When they are both `Int` or both `String`, `T - U` has the values of the value set of `T` that are not in the value set of `U`, and is `Never` when none are left: `String - "a"` is every string but `"a"`, `{"a" | "b"} - "a"` is `"b"`, and `"a" - String` is `Never`. For every other type constructor that is the same in both, `T - U` is `Never`, and `T <: U` must be true (below). Tuples of the same size count as one type constructor, and so do all function types.
- Keep "When `T - U` is `Never`, `T <: U` must be true. …" as it is.
- Add to the examples:
  > `{ Int | String | Nil } - "a" - 1` is `Int - 1 | String - "a" | Nil`.

### §4.5 Subtypes

- In rule 3, replace "every member of `X` has a different type constructor from every member of `B`" with:
  > no value is in both: every member of `X` and every member of `B` have different type constructors, or have the same one and value sets with no value in common. So `"a"` and `"b"` are disjoint, and `"a"` and `String - "b"` are not.
- In rule 4, after "with the same type constructor and equal type arguments" add "and the same value set".
- Add a rule at the end:
  > 8. A member of `Int` or `String` is a subtype of a member with the same type constructor when every value of its value set is in the value set of the other. So `"a" <: "a" | "b" <: String`, and `String - "a" <: String`, but `String` is not a subtype of `"a" | "b"`, and `String - "a"` is not a subtype of `"b"`.

### §4.7 Type inference

Add a bullet:

> - A long union of literal types may be shortened in messages and hovers, as `"a" | "b" | … (40 more)`, but never in a type scheme or in the interface of a package. An inferred type may have literal types in it, also in the interface: an exported constant or an unannotated exported function whose value is a literal has that literal type, so giving one more literal as a value changes the interface. Exported functions should have written types, as the standard library has.

### §4.8 Typing rules

These changes are outside T1's spec list and need approval (§13), but the design cannot be explained without them.

- **Pattern types.** Replace "A literal: the type of the literal." with:
  > - A literal of `Int` or `String`, also `-` and an integer literal: in a clause block with more than one clause or with a guard, its literal type (§4.2); where it is fitted (a binding, a parameter, or a clause block with one clause and no guard), `Int` or `String`, so that every value of that type may come there and the pattern may fail (§7.3). In an item of a list pattern, or inside one, also `Int` or `String`: all the items of a list have one type, so a literal type for one item would be a type for all of them. A float literal: `Float`.
- **Bit-array segments.** Add after "The parts get the types of their formats (§8.2).": "A literal value in a segment also has the type of its format, not its literal type."
- **Rule 12.** Replace the sentence "The catch-all gets the type `B - H`, where `H` is the union of the taken type constructors, each with new type variables as its arguments." with:
  > In the same way, a value `v` of an earlier `Int` or `String` literal pattern at that place, and a member of the type of an earlier type test at that place, is **taken** when every value that the clause pattern matches with it at that place is matched by an earlier clause without a guard. The catch-all gets the type `B - H`, where `H` is the union of the taken members: each taken type constructor with new type variables as its arguments, the literal type of each taken value, and each taken member of a type test. So in `s |> { "a" -> 0  rest -> f(rest) }` with `s: String`, `rest` gets `String - "a"`.
- **Rule 11.** Replace "it takes every value whose type constructor is that of a member of `T` (§7.1), so those values must be in `T`." with "it takes every value whose type constructor is that of a member of `T`, and that, for a member of `Int` or `String`, is in the value set of that member (§7.1), so those values must be in `T`." Add after "If not, then `B <: P`.": "So a clause block with only literal clauses and no catch-all takes only those values: `{ 0 -> "zero"  1 -> "one" }` has the type `(0 | 1) -> "one" | "zero"`. When `B <: P` is not true only because a value of `B` is in no literal of `P`, as `(n: Int) { n |> { 0 -> "zero"  1 -> "one" } }`, the error is the one of §7.4, with a value that no clause matches, and no other error is given for it."
- **Known types.** Replace "A literal has its type" with "A literal, also `-` and an integer literal, also inside groups, has its type (§6.3)".
- **Arguments.** After "Function expressions that have a parameter with no written type are left out of this matching, and come last: their expected types use all the other arguments." add:
  > A function given as an argument may give values to the called function, which the other arguments do not show: as its result or errors, as the argument of a function it is given (a parameter of a parameter), or to a service of its environment. A type variable that is in such a place of a parameter type is matched as usual, and then its type is made whole, both in the expected types of function expressions and in the known type of the call (the bullet on a call `f(a1, …)` above): every `Int` and `String` member, also in an argument of a tuple or a constructor, or in the result of a function type, has all its values, and every other part that is not covariant (a parameter of a function type, an argument of a constructor that is not covariant, an environment) is `_`. So in `list.fold(xs, 0, (n, x) { … })`, `n` has the known type `Int`, not `0`; with `(0, 0)` as the start, `(Int, Int)`; and with `loop: (init: S, body: (S, (S) -> Nil) -> Nil) -> Nil`, in `loop(0, (n, again) { … })`, `n` has the known type `Int`.
- **Matching.** Replace "each type variable in `P` is matched with `K` less the type constructors of the other members of `P` (§4.4). A `_` in `K` matches nothing, and so does a type variable that is generalized in `K`, because it stands for any type." with:
  > each type variable in `P` is matched with `K` less the other members of `P` (§4.4). A type variable in `P` that is matched with a `_` in `K`, or with a type variable that is generalized in `K`, gets `_`, because that part may be any value. So a known type always has every value that may come there, and §6.9 never calls two values disjoint when they may be equal.
- **Names in patterns.** After "take the one member of `B` with the type constructor of `Q`" add: "; for a literal pattern with the value `v`, only that value, if the member has it: `B & v`, with `v` the literal type of the value (not `B & Q`, because where a pattern is fitted `Q` is the whole `Int` or `String`)". So `x = "a" = s` with `s: String` gives `x` the known type `"a"`.

### §6.3 Literals and names

Replace the first sentence with:

> The type of an integer literal is its literal type (§4.2): `1` has the type `1`. When the operand of `-` is an integer literal, also inside groups, the result has the literal type of the negative number: `-1` and `-{1}` have the type `-1`. Every other `-e` has the type `Int`, as `-x` and `-{-1}`. The type of a string literal is its literal type: `"a"` has the type `"a"`. A string with interpolation (`"user-\(n)"`) has the type `String`, because its value is not fixed; it is not a literal, so it may not be a type or a pattern. The type of a float literal is `Float`.

### §6.5 Lists

Replace "`T` may be a union: `[1, "a"]` has the type `List(Int | String)`." with:

> `T` may be a union: `[1, "a"]` has the type `List(1 | "a")`, and may be used where a `List(Int | String)` is needed.

### §6.9 Operators

- Add after the table:
  > The operators take values of literal types, because these are subtypes of `Int` and `String`. Their results have the whole type: `1 + 2` has the type `Int`, not `3`, and `"a" <> "b"` has the type `String`. The one exception is `-` on an integer literal, also inside groups, which has a literal type (§6.3): `-1` and `-{1}` have the type `-1`, but `-x` has the type `Int`.
- In the `==` bullet, replace "So `1 == "a"` and `(s: String) { s == 1 }` are compile errors, but …" with:
  > Literal types count. So `1 == "a"`, `1 == 2`, `1 != 2`, `"a" == "b"`, `x == "b"` after `x = "a"`, and `(s: String) { s == 1 }` are compile errors, but `(s: String) { s == "a" }`, `(m: "a" | "b") { m == "b" }`, `x == Nil` with `x: Int | Nil`, `[1] == []`, `a == b` with a type variable, and `(s) { s == 1 }` are not. To compare two constants when the program runs, for example in a test of `==`, pass them as parameters: `(a, b) { a == b }(1, 2)`.

### §7.1 Grammar

Replace "Only type constructors are tested at run time, so in a `case`, `T` may not have a type variable as a member." with:

> At run time, a member of `T` is tested by its type constructor; a member of `Int` or `String` whose value set is not all the values is also tested by equality (§11.4) with the values in it. So `x: "a" | "b"` takes the strings `"a"` and `"b"`, and `x: String - "a"` takes every other string, but no Int. So in a test, `T` may not have a type variable as a member.

### §7.2 What patterns match

In the table, the row `x: T` becomes:

> | `x: T` | A value whose type constructor is that of a member of `T`; for a member of `Int` or `String`, the value must also be in the value set of the member | `x` |

### §7.3 Patterns that may fail

Add:

> A literal pattern can not fail for its literal type, so `"a" = x` with `x: "a"` needs no `Assert`. Where a pattern is fitted (§4.8), a literal has the whole `Int` or `String`, so `"a" = s` with `s: String` is correct and may fail: it reports to `Assert`.

### §7.4 Covering every value

Replace "So literal patterns of Int, Float, String, and BitArray never cover their type, and a union is covered when every member is covered." with:

> A literal pattern matches its own value, and `"lit" <> r` matches every string that starts with `"lit"`. So literal patterns cover an `Int` or `String` member whose value set is finite, when every value in it is matched: with `m: "GET" | "POST"`, `m |> { "GET" -> 1  "POST" -> 2 }` is covered. Literal patterns alone never cover a member with all its values, or with all but a finite set; a catch-all or a type test such as `x: String - "a"` must take the other values. Literal patterns of Float and BitArray never cover their type. A union is covered when every member is covered.
>
> When the values that are not covered are only values of `Int` or `String` that no literal pattern takes, the type of the clause block already says so (§4.8 rule 11: `(n: Int) { n |> { 0 -> "zero"  1 -> "one" } }` needs `Int <: 0 | 1`). That is reported as this error too, with a value that no clause matches, such as `2`.

### §10.3 Types the host can take

Replace the first paragraph with:

> `Nil`, `Bool`, `Int`, `Float`, `String`, `BitArray`, and custom types with no constructors (§4.3). Where i code gives the value to the host, an `Int` or `String` member counts as `Int` or `String`, whatever its value set (§4.2): a literal type, a union of literal types of one type constructor, or all the values but a finite set, such as `Int - 0`. Where the host gives the value (the value of an external function, the value and errors of a host-service function, and the parameters of a host export), an `Int` or `String` member must have all its values, because the host's values are checked only for their form (§B.2). Values of custom types with no constructors go to the host as opaque references. No other type may go to the host. An error type that the host can take is a union of `Nil`, `Bool`, `Int`, `Float`, `String`, and `BitArray`, with no two the same.

Add this example:

```i
@external("app", "mode")
mode: () -> "on" | "off"           // compile error: the host gives the value
@external("app", "log")
log: (level: "info" | "warn") -> Nil   // correct: i code gives the value
```

## 7. Files to change

T1's files (plan, "各轨道负责的文件"):

| File | Regions |
|---|---|
| `crates/ilang-check/src/types/lits.rs` | new: `ValueSet`, `Lits`, the algebra, `join_all`, `TestValues`, `LitValue`, model tests |
| `types/mod.rs` | `Ty` (:52) field; `Ty::lit`, `Ty::values`; `TestMember`; `type_tests` (:175); size assert |
| `types/solve.rs` | `FailKind::Values` (:38), `constrain_in` (:245, Diff and Con arms), `add_bound` (:385), `route` (:516), `diff_left` (:544), `union_in` (:698), `inter_in` (:744), `merge` (:801), `diff_in` (:877); pass-through in the rest |
| `types/display.rs` | `same_unmarked` (:224), `union_in` (:698), Top rule (near :730), `full` (:753), `inter` (:772), `diff` (:860), `merge` (:889), `tree` (:1176, :1280), rebuilders |
| `types/shown.rs` | `ShownType::Lit` (:10), `text_limited` next to `text` (:46) and `text_with` (:63) |
| `types/known.rs` | `members` (:22), `disjoint` (:64), `match_k` (:322), `ksubst_in` (:382), `call_known` (:413), `member_for` (:434), `written_known` (:550), new `output_vars` and `whole_lits` |
| `types/coverage.rs` | `Vals` (:131), `values` (:173, Diff near :229), `covers` (:267), `useful` (:328, the `Except` arm), `useful_head` (:420), `sub_cols` (:482), `signature` (:518), `Pat::Except`, witnesses and their text (:586) |
| `types/pattern.rs` | `leaf` (:213, whole types at list item places), known filter (:474), rule 11 `sub_case` (:460), `narrow` (:563), W0501 text (:632), type tests (:723–790) |
| `types/infer.rs` | literals (:1076, :1078), unary minus (:1189) and `neg_literal`, `conv` (:852), E0520 text (:1244), `call` (:1308–1321, the second fix of decision 7), `sub_case` next to `sub` (:747), host export checks (:440–560) |
| `crates/ilang-wasm/src/lower.rs` | type test (:2128); `head_cond` (:2211) unchanged |
| `docs/spec.md` | §3, §4.2, §4.4, §4.5, §6.3, §6.9, §7.1, §7.4 |

Outside T1's list (needs approval, §13):

- `docs/spec.md` §4.1, §4.7, §4.8, §6.5, §7.2, §7.3, §10.3.
- `crates/ilang-syntax`: the `Literal` Prim in the type parser, the formatter and the printer; `ast::TypeKind::Lit` (A1's crate).
- `crates/ilang-check/src/hir.rs`: `hir::TypeKind::Lit`; `resolve.rs` `fn ty` (:1329), `host_type` (:1651), `host_part` (:1571), `host_fns` (:1473).
- `docs/design/meta.md` §2.3 (a Types row), §4.3 (macro arguments in a type), §7.4 (`members`), and the Lisp parser and printer (M4's), as §5.9 says.
- `crates/ilang-check/src/query.rs` (:1218) literal case.
- `crates/ilang-cli/src/commands/package.rs` (:1165, :1351).
- Tests: `crates/ilang-check/tests/codegen.rs` (:108), `tests/query.rs`, the goldens in `crates/ilang-check/tests/types`, and `tests/spec`.

## 8. Changes to existing tests

The prototype ran every case with both binaries. T1 should see the same:

- **Checker goldens** (`crates/ilang-check/tests/types`): 30 of the 42 `.out` files change, 118 of 293 type lines and 78 message lines (mostly `Int` → `1`). New diagnostics: E0520 at `spec.i:71` (`assert(1 == 2)`) and `known_intersection.i:5` (`inc(5) == 1`), and one W0501. Six E0516 go away, because literal clauses on finite sets are now covered (`digit`, `ints`, `strings`, `Ok(1) | Ok("a")`). Each changed type line is read by hand, not only re-blessed. `guarded_only` now infers `x: 1`; that is the principal type.
- **`tests/query.rs`:** two string checks expect `-> Int` and now see `-> 1`.
- **`tests/spec`:** E0520 in `11/equality` (`1 == 2`, two different big ints, `"a" == "A"`), `6/strings` (`"a" == "b"`), `11/stop_tail_service` (`v == 0` with `v: 1`) and `6/blocks` (a local constant `== ""`). Each is a run-time test of `==`; move the values behind parameters, `(a, b) { a == b }(1, 2)`, so the test still checks `==` when the program runs. W0501 in `6/blocks`, `9/control` and `9/control_dropped`: a match on a constant subject, so `_ ->` is never used; move the subject behind a parameter the same way.
- **std, examples, bench, projects:** no new diagnostics. 10 of 1787 printed type lines in std change, all top-level constants (`hex_digits : "0123…F"`, `sign_bit : 9223372036854775808`). `i test` on std: 1386 passed.

T1 counts all of this again at the switch-on step and compares it with these numbers. A count above them goes to the coordinator before the step is merged.

## 9. Performance

Measured on the prototype (single runs on a busy machine; re-measure in a quiet window):

- `i check` on std: base median 160 ms, prototype 189 ms, **+18%**.
- Stress package, n = 1000: `keys` (a list of n distinct strings) 14 → 544 ms; `pick` (a match with n string clauses) 122 → 810 ms. Both grow with n² in the prototype.
- `calls` (n calls of one local function) is already about 13 times slower when n doubles in the base. That is a base-checker problem, not S2's: **it gets its own ticket.**

Causes and fixes, in the order T1 applies them:

1. **k-way join.** Building a union of n literals one at a time is n merges of growing sets. `join_all` in `union_in` and in `Norm` makes it one merge. This is the n² in `keys` and `pick`.
2. **Bound fusion** (§5.1 rule 8), lower and upper. Without it a variable has one bound per distinct literal, and each new bound is compared with all of them.
3. **Cheap Eq and Hash.** The cached hash and `Arc::ptr_eq` (§4) make solver cache keys, `out.contains` and `Norm`'s 64-step loop O(1) per set in the common case. This is in the representation from the start.
4. **Narrowing fast path** (§5.3): no `useful()` call per literal for the common `"a" -> …  rest -> …`.
5. **Interning** (a per-check `SetId(u32)`) only if 1–4 are not enough.

**Budget.**

- std `i check` median: at most +10% over 1865366, never worse than the prototype's +18%. Measured with `time-std.sh` (11 runs, one warm-up), base and T1 one after the other in a quiet window.
- `stress-time.sh` at n = 250, 500, 1000: T1 at most 2× base at n = 1000 for `keys` and `pick`, and no part grows more than 2.5× from n = 500 to n = 1000 (n² growth is 4×).
- Ignored stress tests (`#[ignore]`, run by the perf script): a match with 2000 string clauses; one function called with 5000 distinct strings; a 1000-literal alias used in 100 functions; a nested match over `(String, String)` with 50 × 50 literals. Each checks, through a debug counter, that no variable has more than 2 bounds of one head.
- Codegen: a type test on N strings allocates N strings each time. Accepted in T1; hoisting is B's follow-up.

The scripts are in `/home/agent/Programming/i/.rd/spike-s2/spike/`: `time-std.sh`, `stress-time.sh` with `gen-stress.py`, `check-all.sh` with `compare.sh`, and `diag-diff.sh`.

## 10. Tests

**`lits.rs` unit tests.**

- Exhaustive model test: a universe `{v0, v1, v2, v3} ∪ {★}`, where `★` stands for every other value. `F(X)` maps to `X`, `E(X)` to `(U ∖ X) ∪ {★}`, `All` to `U ∪ {★}`. For all 16 values of `X` in each of the 3 kinds, every op (union, meet, minus, subset, disjoint, contains, `join_all`) agrees with the model.
- Canonical form: `F` is never empty, `E(∅)` is `ALL`, items are sorted with no duplicates, `Eq` agrees with set equality, and two equal sets in different `Arc`s are equal with equal hashes.
- Literal values: `0x10` is `16`, `-0` is `0`, `"\u{e9}"` is `"é"`, `"e\u{301}"` is not `"é"`.

**Solver unit tests** (solve.rs `#[cfg(test)]`, Tys built directly):

- `"a" <: "a" | "b"`, `"a" | "b" <: String`, `String </: "a"`, `String - "a" </: "b"`, `"b" <: String - "a"`, `"a" </: String - "a"`.
- Route: `String <: "a" | v` gives `v ≥ String - "a"`; `String <: "a" | v | w` is Ambiguous; `String <: "a" | Nil` is Mismatch.
- `diff_left`: `(v - "a") <: R` after `v ≥ String` sends exactly `String - "a"` to `R`; all three cases (subset, disjoint, both).
- `"a" & "b" = Never`; `("a" | "b") & (String - "a") = "b"`; `String - "a"` is cofinite; `"a" - String = Never`; `"a" | String - "a" = String`.
- Bound fusion: 1000 distinct string lower bounds leave one bound, and an upper bound `"x" | "y"` still rejects the first bad one. Two upper bounds `"a"` and `"b"` stay two bounds.
- Env mode (rule 1, sets must be equal): `f = (c ! Counter(0)) { … }` called under `! Counter(Int) = …` is rejected with E0515 (`FailKind::EnvForms`), because `0` and `Int` are different sets; `Counter(Int)` on both sides is accepted; `! Counter(Int) = counter.start(0)` is accepted, because the written type is the type of the environment and `0 <: Int` is an ordinary constraint on the argument. A test for each of the three.

**Display unit tests:** `"a" | _ - "a"` and `0 | _ - 0` show `_`; `_ - String | "a"` does not; `String - "a" - "b"` sorted; `Int - -1`; Ints sorted by number (`-2 | 1 | 10`); a 40-literal union shortened to `… (30 more)` in a message and full in `text()` and the package interface.

**Spec tests** (`tests/spec/<section>/`, in the existing directories). Every "must reject" case is a program that would go wrong when run if it were accepted. Run tests run at O0 and O2.

- `6/literal_types.i`: `x = "a"` is `"a"`; `-1` and `-{1}` are `-1`; `-x`, `-{-1}` and `1 + 1` are `Int`; `-(1)` is rejected (a tuple); `[1, 2]` is `List(1 | 2)` and is accepted where `List(Int)` is needed; `[1, "a"]` is `List(1 | "a")`.
- `4.8/known_fold.i` (run): `count = (xs: List(Int)) { list.fold(xs, 0, (n, x) { n == 3 |> { True -> n  False -> n + 1 } }) }` gives no E0520 and stops at 3 on a list of five; the same with `index_fold` and a loop helper written in the test; `list.fold(xs, State(0), (s, x) { State(s.count + 1) })` still knows `s.count`; `list.map(["a"], (s) { s == "b" })` still gives E0520.
- `4.8/known_output.i` (run), one case for each output place of decision 7, each `True` or stopping when run and each giving neither E0520 nor E0501: a tuple accumulator, `fold(xs, (0, 0), (acc, x) { acc.0 == 3 |> { True -> acc  False -> (acc.0 + 1, x) } })`; a generic record accumulator, `Box(0)` with `.value`; a parameter of a parameter, `capture(0, (k) { k(5) }) == 5` and `capture((0, 0), (k) { k((5, 5)) }).0 == 5`, with `capture: (x: A, f: ((A) -> Nil) -> Nil) -> A` written with `control.boundary`; the loop `loop(0, (n, again) { n == 3 |> { True -> Nil  False -> again(n + 1) } })` with `body: (S, (S) -> Nil) -> Nil`; and a service argument, `step: (S) -> Bool ! State(S)` that calls `set(n + 1)`.
- `4.8/known_group.i`: `pair = (1, f)` and `f = () { pair.0 == 2 }` in one group gives E0520, as it does outside the group.
- `7/literal_missing.i`: `(n: Int) { n |> { 0 -> "zero"  1 -> "one" } }` is E0516 with the witness `2`; `(r: Result(Int, Nil)) { r |> { Ok(0) -> 1  Error(_) -> 2 } }` is E0516 with the witness `Ok(1)`; `(n: Int | Nil) { n |> { 0 -> "zero"  Nil -> "none" } }` is E0516 with the witness `1`. None is E0501, and each gives exactly one diagnostic.
- `7/literal_list.i` (run): `(args: List(String)) { args |> { ["git", "push"] -> 1  [] -> 2  [_, ..] -> 3 } }` and the same with `[0, 0]` on `List(Int)` are correct and give no diagnostic; `(xs: List("a" | "b")) { xs |> { [] -> 0  ["a", ..] -> 1  ["b", ..] -> 2 } }` is covered.
- `4.8/literal_match.i`: `kind = { "a" -> 1  "b" -> 2 }` has the type `("a" | "b") -> 1 | 2`, and `kind("c")` is rejected. With `_ ->` it shows `(x) -> …`.
- `4.8/literal_narrow.i`: `s |> { "a" -> 0  t -> g(t) }` with `g: (String - "a") -> Int` is accepted; with `g: ("b") -> Int` it is rejected.
- `4.8/literal_narrow_nested.i` (run, O0 and O2): `(r: Result(String, Nil)) { r |> { Ok("a") -> 0  Ok(rest) -> g(rest)  Error(_) -> 1 } }` with `g: (String - "a") -> Int` is accepted; `(p: (String, Bool)) { p |> { ("a", True) -> 0  (t, _) -> g(t) } }` with the same `g` is rejected, because `t` stays `String` (`("a", False)` comes to the second clause); `(p: (String, Bool)) { p |> { ("a", _) -> 0  (t, _) -> g(t) } }` is accepted. The same three with a type test `x: String - "a"` at the nested place in place of the literal clause.
- `4.8/known_unknown_part.i`: `result.unwrap(Ok(1), 50) == 1` gives no E0520 (regression for decision 7).
- `7/literal_type_test.i` (run): `x: "a" | "b"` does not match `"c"` or `1`; `x: String - "a"` does not match `3` or `"a"` and matches `"b"`; `x: Int - 5` does not match `5` or `"5"` and matches 2^30, −2^30−1 and 10^30; `x: 1073741824` matches only that number.
- `7/literal_cover.i`: `s |> { "" -> 0  "a" <> _ -> 1 }` on `String` is not covered, and the witness is not `"a"` (it is `"b"`); `(m: "\u{0}" | "a") { m |> { "a" -> 1 } }` gives the witness `"\u{0}"`, as the type prints it, not `"\0"`; `"a"`, `"b"` cover `"a" | "b"`; they do not cover `String` (witness: `""`) or `String - "c"`; `"a" <> _` and `"b"` cover `"ab" | "b"`; `"a"` and `x: String - "a"` cover `String`; `"a"` and `x: String - "a" - "b"` do not (witness `"b"`); `"a"` and `x: String - "a"` cover `m: "a" | "b" | "c"`; `(x: String - "a", _)` and `("a", True)` do not cover `(String, Bool)` (witness `("a", False)`); a `"a"` clause on `String - "a"` is never used; Float literals never cover. Each case that is not covered is also a run test that the checker must reject; each covered case runs at O0 and O2 with the values in every `X`.
- `4.8/literal_narrow_except.i`: `s |> { x: String - "a" -> 0  t -> g(t) }` gives `t` the type `"a"`; `s |> { 1.5 -> 0  y -> f(y) }` and `s |> { "a" <> _ -> 0  t -> g(t) }` take nothing (`y: Float`, `t: String`).
- `7/literal_may_fail.i` (run): `"a" = s` with `s: String` needs `Assert` and fails with `"b"`; `"a" = x` with `x: "a"` needs no `Assert`; a segment `<<"hi": utf8, rest: bits>>` against a decoded value is accepted.
- `6/eq_literal.i`: errors for `x = "a"  x == "b"`, `1 == -1`, `-{1} == 2`, `1 != 2` (the message says "always `True`"), `(m: "a" | "b") { m == "c" }` (the message does not call `m` a constant), and the shortened message for a 40-literal union; no error for `(s: String) { s == "b" }`, `(m: "a" | "b") { m == "b" }`, `(s) { s == 1 }`, and `n == 0x0` with `n = 0`.
- `10.1/external_literal.i`: `@external … mode: () -> "on" | "off"` is rejected; `log: (level: "info" | "warn") -> Nil` is accepted and is called; so is `log: (level: Level) -> Nil` with `Level = "info" | "warn"`; a host-service function with a parameter `"a" | "b"` is accepted and one with the value `"a" | "b"` is rejected.
- `10.2/export_literal.i`: a host export whose parameter is inferred as `"a" | "b"` is rejected; one whose value is `"ok"` is accepted; `f = (n: Int) { n |> { 0 -> 1  m -> m } }`, whose value is `Int - 0`, is accepted.
- `6/interpolation_type.i` (after A1 adds interpolation): `id = (n: Int) { "user-\(n)" }` has the type `(n: Int) -> String`; `"a\(x)"` in a type and as a pattern is a compile error.
- Principal types: `id = (x) { x }` at `"a"` gives `"a"`; `pick = (b, x, y) { b |> { True -> x  False -> y } }` at `"a"`, `"b"` gives `"a" | "b"`.

**Metaprogramming (M4, after both T1 and M4 land):** a `.i` file with `(m: "GET" | "POST")` and `Int - -1` changed with `i fmt --to=is` and back is the same code; `to_text(1)` and `to_text("x")` give the `Int` and `String` branches; `members(type_of(m))` with `m: "GET" | "POST"` gives two items; a type macro called as `m(-1)` gets `(- 1)`.

**`tests/codegen.rs`** (:108): expect `TestMember`; add one finite and one cofinite case.

**Corpus.** `check-all.sh` and `compare.sh` over std, examples, bench, `tests/spec` and the checker's project tests, before and after the switch-on step (§8).

## 11. Risks

1. **More E0520.** No widening means `1 == 2`, `x = "a"` then `x == "b"`, and `f() == 2` where `f` gives a constant are compile errors. The prototype found 6 in `tests/spec`, 2 in the goldens and none in std. In-flight branches (T2, Std, R, IO, json, http) may hit more after rebase; the plan's RF round after T1 counts them. A fallback, if this is too many: known types of function results use the whole type constructor. That is a spec change and needs the user's approval. Known types that are too small give *false* E0520, which is worse than more E0520: the paths found so far are the `_` part (`unwrap`), and the accumulator of a loop helper (`fold(xs, 0, …)`), both fixed by decision 7. The RF soundness view looks for more paths where a known type comes from some of the values only.
2. **Inferred types change.** A literal-only match infers a literal union (`digit : (x: 0 | 1) -> "one" | "zero"`). Callers that pass other values now get a type mismatch where they got a coverage error before. Unannotated exports show literal types in their interface (§4.7 text). An invariant parameter (`Boundary`, or a custom type that uses a parameter both ways) written as `Codec(String)` will not take an inferred `Codec("a")`; the user writes `String` where the value is made.
3. **Rebuild sites.** 77 `Ty::Con` sites must carry the set. A site that writes `ALL` by mistake silently widens a type; where it widens an upper bound, that is unsound. The compiler does not stop at the 8 `Ty::Con(..)` sites or at the `.head()` comparisons (decision 2), and these are where most set rules belong. Step 3 of the checklist is reviewed site by site from a search, not from compile errors.
4. **Route and `diff_left` splits.** A wrong split (for example sending the whole member to the same-head member) makes `String <: "a" | r` fail, and fixing that by over-approximation could let a value escape narrowing. The model-checked algebra and the solver tests guard it, and the RF soundness view aims at it.
5. **Coverage.** `Prefix` and `Except` rows in finite signatures and default matrices are new interactions, and `Except` has no prototype evidence. A mistake toward "covered" is a run-time failure. Dedicated tests at O0 and O2.
6. **Host boundary.** Without §10.3's new sentence, external values and export parameters with literal types are a hole. Host exports need no written types (§10.2), so their parameter types are inferred, and they get literal types as soon as literal values do: `handle = (m) { m |> { "GET" -> 1  "POST" -> 2 } }` is inferred as `("GET" | "POST") -> …`, has no `_`, and traps at O0 (or takes a wrong clause at O2) when the host gives `"PUT"`. So the host-export check (`infer.rs` `host_type` / `host_part`) lands in the same commit as literal values (step 7). Externals and host-service functions always have written types, so their check lands with written literal types (step 8).
7. **Known types.** Giving `_` for a generalized variable as well as for `_` (decision 7) goes slightly past the prototype. It could make a known type less exact and give a new E0506 (needs a known type). The corpus count catches it.
8. **Performance.** See §9. If the budget is not met after the five fixes, the coordinator decides.
9. **Display.** Shortened text must never reach package identity or `Typed::schemes`. The Top rule must use the subset check, or ordinary code prints `"a" | _ - "a"`, or worse, `_ - String | "a"` prints as `_` and hides a real constraint.
10. **Syntax timing.** `-` in types is also the difference. The parser, the formatter and the printer must agree that `-` followed by an integer literal at the start of a `Prim` is a sign, so that `Int - -1` round-trips. That is in A1's crate.
11. **Metaprogramming.** If M4 lands before the §5.9 rows, a `.i` file with a literal type cannot become `.is`, and typed macros get types that `subtype` and `members` call a `SyntaxError`. If T1 lands first, the same is true of M4's first version. Either order works only if §5.9 is in `meta.md` before M4 writes its type rows.
12. **Coverage error text.** Rule 11 now finds most missing literal values before coverage does. `sub_case` drops a failure of kind `FailKind::Values` and lets coverage report it. If the kind is set too rarely, users see "`Int` is not a subtype of `0 | 1`" where they saw a missing value. A failure that coverage does not find is still reported, as E0501, because `sub_case` only holds failures back until coverage has run. `7/literal_missing.i` pins the text and the count of diagnostics.

## 12. Implementation checklist for T1

Each step is one commit and can be tested in this repo. Steps 2–6 change no behaviour that a user sees: before step 7 nothing makes a set other than `ALL`, and every rule that could make one is written but not reached, or lands in step 7. In particular, the rule 12 literal candidates (which make `Ty::lit` members from coverage's `Lit::Int` and `Lit::Str`, and so cofinite catch-alls such as `String - "a"`) and both `match_k` fixes (which change known types at 1865366 too, risk 7) land in step 7, where §8's count is made. Each of steps 2–6 checks that the goldens, `query.rs` and the spec tests are unchanged, with no re-bless.

1. **Approvals and spec.** Get the coordinator's approval for the items in §13 (the extra spec sections, the parser branch, `resolve.rs`, the `meta.md` rows of §5.9). Send §5.9 to M4, so that its type rows and `members` have literal types from the start. Apply §6 to the converted `spec.md`. If the parser's `Literal` Prim has landed with A1, every ```i block parses. If not, the blocks that write literal types (§3, §4.2, §4.4, §4.5, §7.1, §7.4, §10.3) are marked as not yet checked, and step 8 removes the marks.
2. **`lits.rs`.** `ValueSet`, `Lits`, the algebra, `join_all`, the canonical constructors, `TestValues`, and the model tests. Share the canonical Int text with `coverage::literal` (:672).
3. **The field.** `Ty::Con(Head, ValueSet, Vec<Ty>)`, `Ty::lit` with its debug assertion, `Ty::values`, the size assert. Pass the set through at every site; review each rebuild. Then search for `Ty::Con(..)` and `.head() ==` / `.head()` comparisons in `types/` (the sites of decision 2, and any new ones) and write the list in the commit message, each site marked "set rule in step 4/5/6" or "the set does not matter here, because …". `type_tests` becomes `Vec<TestMember>`; `lower.rs` maps `All` to `head_cond` and has the `Only` and `Except` branches written but not yet reachable. All tests and wat snapshots stay the same.
4. **Solver** (§5.1 rules 1–10) with its unit tests and the bound fusion.
5. **Display, known types, coverage.** `Norm`, the Top rule, `same_unmarked`, `tree`, `ShownType::Lit`, `text_limited`, `package.rs`; `known.rs` with `disjoint`, `members` by set, `whole_lits` and `output_vars` (with unit tests, not yet called), and `written_known` with `neg_literal` (reached only by literal values); coverage with sets, `other` as all values, finite signatures, `covers()`, `Pat::Except`, the listed-values rule of `useful_head` and the `Except` arm of `useful`, `sub_cols`, witnesses and their §1.6 text. Unit tests that build types directly. The witness text changes no golden unless a golden has a string witness with an escape; if one does, it is the only re-blessed line, and the commit message names it.
6. **Patterns.** Type-test members (all `ALL` until step 8), the known filter by `known_diff`, `FailKind::Values` and `sub_case` for rule 11 (no failure has the kind yet), the W0501 text, and the rule 12 code for literal candidates with its fast path, behind one `const LITERAL_CANDIDATES: bool = false`. `leaf` keeps the whole types for now.
7. **Switch on literal values** (one commit, easy to revert). Literal expressions and `-` on an int literal in `infer.rs`, and interpolated strings as `String`; both `match_k` fixes, and the second fix of decision 7 (`output_vars` and `whole_lits`) in both `call` and `call_known`; `LITERAL_CANDIDATES` on; `leaf` gives literal types in a match (not when fitting, never in a segment, never at a list item place); the E0520 text; and the host-export part of §5.8 (`infer.rs` `host_type` / `host_part`: a parameter must have all values), because export parameters are inferred and get literal types in this step (risk 6). Re-bless the goldens and `query.rs` and read each change. Rewrite the spec cases of §8. Add the tests of §10 that need no written literal type, including `unwrap(Ok(1), 50) == 1`, `known_fold.i`, `known_output.i`, `known_group.i`, `literal_missing.i`, `literal_list.i`, the literal cases of `literal_narrow_nested.i` and `10.2/export_literal.i`. Run `check-all.sh` and compare with §8; a higher count goes to the coordinator.
8. **Written literal types** (one commit): the `fn ty` branch and `hir::TypeKind::Lit` (and the parser branch, if A1 did not do it), the Float error, the host boundary for externals and host-service functions (§5.8, `resolve.rs`, with the union-of-literals case), and the `lower.rs` equality tests now reachable. Run tests at O0 and O2 for every type-test case of §10. Remove the "not yet checked" marks of step 1.
9. **Performance.** `time-std.sh` and `stress-time.sh` against §9's budget; the ignored stress tests. Apply interning only if needed. Open the ticket for `calls`.
10. **Blind spec tests and RF**, last, as the plan says. The soundness view writes programs that type-check and still go wrong when run, at O0 and O2, aimed at the route split, `diff_left`, `Except` coverage, the host boundary, and i31 against boxed Ints.

## 13. Open questions

1. **Scope.** §4.1, §4.7, §4.8, §6.5, §7.2, §7.3 and §10.3 are not in T1's spec list, but the design needs them. May T1 change them? §4.7 and §4.8 are also T2's.
2. **Parser.** The `Literal` Prim in types (`ilang-syntax` parser, formatter, printer, `ast::TypeKind::Lit`) is A1's crate. Should A1 add it before T1 starts, or may T1 change those lines?
3. **`resolve.rs`.** The host checks (`host_type` :1651, `host_part` :1571, `host_fns` :1473) and `fn ty` (:1329) are in `resolve.rs`, which T1 does not own. Approve T1 changing those regions?
4. **Host values.** This design rejects literal types where the host gives a value. The other choice is to accept them and stop the program when the host's value is not in the set, as §B.2 does for a bad form. That is more general, but it changes C's `emit/host.rs`. Keep rejection for now?
5. **E0520 on function results.** If the RF count after T1 is high, should known types of function results use the whole type constructor (risk 1)? That is a spec change for the user.
6. **Known types of generalized variables.** Decision 7 gives `_` for a generalized type variable as well as for `_`. The prototype only did `_`. Keep both, or only `_` until a case shows the need?
7. **Metaprogramming.** §5.9 changes `meta.md` §2.3, §4.3 and §7.4, which are M4's. Should M4 take these rows as written, and is `members` giving one item per literal value (and not one per §4.4 member) the right API, or should `i/syntax` get a separate `values(t)`?
8. **The accumulator of a loop helper** (decision 7, second change). This makes `Int` and `String` members whole only. A callback that gives back another type constructor than the start value still gets a known type that is too small, as at 1865366. Fix that in T1 too (the variable gets `_`, which loses field access on records in `fold`), or in its own ticket as proposed?
