# Design: structural records

Status: draft for review (study S3 of the redesign plan). When it is accepted at gate G3, track T3 puts the spec text of part 12 into `docs/spec.md` and makes the changes of parts 14 to 16. T3 starts after A1 (the new parser) and T1 (literal types) have landed, and after T2 has merged. Code facts are from commit 1865366. Line numbers are at that commit; A1, T1 and T2 move many of them.

The examples use the new syntax of `docs/design/syntax.md`.

## 1. The problem

i needs records with fields that are not declared in advance:

```i
ada = { name: "ada", age: 36 }          // { age: 36, name: "ada" }
name_of = (r) { r.name }                 // (r: { name: A }) -> A
name_of(ada)                             // "ada"
```

A literal has T1's one-value type, so a record literal has a record of one-value types: `{ age: 36, name: "ada" }` is a subtype of `{ age: Int, name: String }`. The examples below show these types where they matter.

The plan asks for MLstruct-style records:

- A record type `{ name: String, age: Int }` is the intersection of one-field record types, `{ name: String } & { age: Int }`.
- Width subtyping: a record with more fields may be used where fewer are needed.
- Keys are String literal types (from T1). So reading and setting a field by key are ordinary library functions, `record.get(r, "name")` and `record.put(r, "age", 37)`.
- A library such as a schema library is plain i code that builds a record type field by field with `|>`.

The hard parts are these:

1. **Soundness of `put`.** `put(r, "age", v)` replaces a field that is already there. Typing its result as `r & { age: T }` is unsound (part 5.6).
2. **Keys whose value is not known.** Inside a generic function, the key is a type variable. A union key or a plain `String` key has no single field.
3. **The solver's head rule.** The solver assumes that two members with different type constructors have no values in common. If every label were its own type constructor, `{ a: Int } & { b: Int }` would become `Never`, and `Never` fits everywhere. That would make every program correct.
4. **`v.f` today** needs a known type and reads a field of a custom type (§4.8 rule 13). Records need a principal rule: a fresh `A` and `t <: { f: A }`.
5. **Syntax.** `{` already begins blocks, clause blocks and type groups.
6. **Run time.** WasmGC types are fixed when the program is compiled, but `put` makes new shapes while the program runs. `==`, `hash` and `inspect` must agree with each other and must not depend on how a record was built.

## 2. The decisions

Each decision has its reason. The exact rules follow in parts 3 to 10.

**D1. All records are one type constructor.** There is one new head, `Head::Record`, and one run-time tag, 14. A record type is one canonical form, `Ty::Rec`: fields sorted by label. It means the MLstruct intersection of one-field records, so `{ a: A } & { b: B }` is `{ a: A, b: B }`.
*Reason:* the solver's rule "different heads are disjoint" is used by `inter_in`, `route`, `-`, type tests, `known::disjoint` and narrowing. With one head it stays true for records with no exception, and two records in one `&` merge by label instead of becoming `Never`.

**D2. Width subtyping is one new rule, `Rec <: Rec`.** A union of two records is put together into their common labels (the MLstruct join), the same way §4.4 puts `List(Int) | List(String)` together. A spread type whose labels are not all known (its base is not a record type, as in `{ ..R, a: Int }` with a type variable `R`, or one of its labels may be a label parameter) cannot be put together with a record type. It stays a member of its own, next to at most one record type (part 4.5). A union with such a spread member may only be in an output place, the same rule as §4.4 gives a union with two type variables (E0518, part 8.2). The spec says so in §4.4 and §4.9.3.
*Reason:* it keeps "at most one `Rec` member in a union", which `route` and known types rely on. A spread member is never merged, and a record goes to the `Rec` member first, or to the one spread member when there is no `Rec` member. That choice may miss a member the record is in, so, as for two type variables, inference cannot choose well between such members, and the union is kept out of input places, where a correct call would be rejected. One arm (4.4, arm 5a) sends a type with a spread part to a spread member of a union, chosen by its labels, so that such a union can be written as a result type.

**D3. "Forgetting" is one exact type form, `Ty::Without(T, L)`.** It means "the records of T, with the labels in L forgotten". It holds only records. The written form is a spread: `{ ..R, age: V }` is `Without(R, {age}) & { age: V }`, and `{ ..R, -age }` is `Without(R, {age}) & {..}`. There is no `\` operator and no new token.
*Reason:* `put` and `remove` need a type for "R, except that this field may now be anything". `Without` has an exact rule on the left of `<:` and a sound rule on the right. It is rewritten only by equalities (part 4.3). So no rule depends on whether a type is in an input or an output place, and the coalescing and simplifying in `display.rs` (which make the schemes of generalized bindings, `infer.rs:2108`) cannot make it unsound.

**D4. Keys are label parameters.** A type variable declared on a binding and used as a label (`{ K: V }`) in the binding's written types is a label parameter. A declared variable that is only the type of a parameter, as `A` in `id(A) = (x: A) { x }`, stays an ordinary type variable. As a type, it may only be the whole type of a top-level parameter of the binding's own function (`key: K`), its **key parameter**. At each call it is fixed from the key argument: an argument whose known type is exactly one String literal, which the solver then checks against that one-value type, or the key parameter of a binding around the call, passed by its name. Union keys, plain `String` keys and keys with no known type are compile errors. A name bound by a binding with label parameters may only be the callee of a call, also inside its own value and in a type of a value (L5); a key parameter may not be named in a type of a value (L2a); and the binding may not be in a recursive group with other bindings. Inside the binding, a type that holds the label parameter may not reach a type variable of a name from outside the binding (L8). A binding with label parameters may not be an entry point or a host export, because the host calls it with no check of its keys (L9). The solver never sees a label that is an inference variable.
*Reason:* this is the fallback rule of the task, made general: any library may declare label parameters, and `i/record` uses nothing else. Inference stays principal, because labels are fixed before solving and never by solving. The limits on where `K` may be a type are what tie the value of the key to its label (part 5.5).

**D5. Inside its own binding, a label parameter is rigid.** `K` is equal only to itself for `<:` and `&`. For forgetting, `K` may be equal to any label, so `{ ..{ a: Int }, K: V }` keeps `a` only as a possibility: reading `.a` there is the compile error "the key K may be `a`".
*Reason:* both rules stay sound after `K` is replaced by any string (part 5.5).

**D6. `v.f` has three rules.** 13a: when every member of the known type of `v` is a custom type (also when it has no members, `Never`), the old rule applies, with `Access::Field`. 13b: when `v` has no known type, or its known type has members and every member is a record type, `_` or a type variable: a fresh `A` and `t <: { f: A }`, with the new `Access::Label`. 13c: tuple parts, unchanged. A known type that mixes custom types and other types is a compile error.
*Reason:* every program that is correct today keeps its meaning and its code. 13b is principal.

**D7. Custom types are not records.** A `User` is never a subtype of `{ name: String }`.
*Reason:* the values have different run-time tags, so the sets of values are disjoint. A record view would break D1, would need a run-time dispatch for every `.f`, and is wrong for a type with several constructors when only some have the field.

**D8. Run time: one `$obj` with tag 14, labels and values interleaved.** Field 5 holds `[label0, value0, label1, value1, …]`, sorted by the bytes of the labels.
*Reason:* the shapes that reach a function are open (width), so one struct type for each shape would need a test for each shape anyway. With this layout the existing generic `$obj` paths of `$value_compare_i32` and `$value_hash_u32` already give the right `==` and `hash`. Only `inspect` needs a branch.

**D9. Syntax.** A `{` is a record when it is `{..}`, when `..` follows it, or when its first part is `label :` and either has no top-level `=`, `->`, `?` or `|`, or is followed by a `,`. Otherwise a clause head or a binding wins. The same scan is used for a function body, so `(n) { name: n }` gives a record. `{..}` is the empty record and the type of all records. `{}` stays only the empty clause block.
*Reason:* one rule for every `{`. No program that is correct today changes, because a block or clause block never starts with `label :` without one of those four tokens, and never has a `,` directly inside its braces. The `,` gives every record a written form, also a record with one field that holds a function with a `->` part. `{..}` reads back as itself, so the text form of the empty record is i source.

**D10. Out of T3:** record patterns, union keys, a record view of custom types, and the Lisp-surface rows (proposed to M1 in part 13). A type test `x: { … }` is in T3: it takes every record, by head.
*Reason:* patterns touch `pattern.rs` and `coverage.rs`, which T1 owns, and type tests with rule 11 already give a sound way to tell records from other values.

## 3. What a record type means

Types are sets of values.

- A **record** is a value with fields. Each field has a label, which is a String, and a value. No label is in one record twice. The order of the fields has no meaning.
- `{ f1: T1, …, fn: Tn }` is the set of records that have **at least** the fields `f1 … fn`, with the value of `fi` in `Ti`. Other fields may hold anything. It is the same as `{ f1: T1 } & … & { fn: Tn }`.
- `{..}` is the set of all records.
- **Forgetting.** `Without(T, L)` is the set of every record `x` for which some record `y` in `T` has the same fields as `x` at every label that is not in `L`. Fields at labels in `L` may be there or not, with any values. It holds only records: the values of `T` that are not records are not in it. So `Without(T, L)` holds every record of `T` and more. It is **not** "remove the field": a value of `Without({ a: Int }, {a})` may still have a field `a`. With no labels, `Without(T, {})` is `T & {..}`. This is a rule on sets: two types with the same values give the same result. So when `T` has no values, `Without(T, L)` has none: `Without({ a: Never, b: Int }, {a})` is empty, like `Without(Never, {a})` (rules E1 and E4 of 4.3).
- `{ ..R, f1: T1, …, -g1, … }` is `Without(R, {f1, …, g1, …}) & { f1: T1, … }`. When there are no `fi`, the second part is `{..}`. `{ ..R }` is `R & {..}`.
- A record type with a field of type `Never` has no values. It is kept as it is and not turned into `Never`, the same as `(Never, Int)` today. That can only make the solver accept less. Forgetting a label of it gives `Never` (E1 of 4.3), because it has no records.

## 4. Types in the checker

### 4.1 Labels

```rust
pub enum Label { Name(Arc<str>), Param(LabelParamId) }
```

- `Arc<str>`, not `Rc<str>`: the LSP sends `Checked` (which holds `Typed`, its `LocalTypes` and the `Solver`, and so every `Ty`) to another thread (`ilang-lsp/src/analysis.rs:79`, :99). `Ty` must stay `Send`.

- Order: every `Name` before every `Param`; `Name`s by their UTF-8 bytes; `Param`s by id. The order of `Name`s is the order of `$string_compare` at run time (bytes, then length), which is Rust's `str` order.
- `Param` labels exist only inside the binding that declares the label parameter, and in its type scheme. Every call replaces them (part 5). L8 (part 5.2) rejects a `Param` label that would reach a type variable from outside the binding. Because a binding with label parameters is never in a recursive group with another binding (L6), no other binding sees its type before it has a scheme.
- Two labels are **the same** when they are equal. They are **different** when both are `Name`s and not equal. Otherwise (at least one is a `Param`, and they are not equal) they **may be equal**.

### 4.2 Forms

In `crates/ilang-check/src/types/mod.rs`:

- `Head::Record`: a new variant with no data, so `Head` stays `Copy`.
- `Ty::Rec(Vec<(Label, Ty)>)`: fields sorted by label, no label twice. `head()` is `Some(Head::Record)`. Every field is covariant.
- `Ty::Without(Box<Ty>, Vec<Label>)`: the labels sorted, none twice, at least one. `head()` is `None`, because it is not a form that R-width can read, and because a union may hold it next to a `Rec` (part 4.5). `is_varlike()` is true, as for `Diff`. The base is covariant.
- A helper `Ty::value_head()`: `head()`, but `Some(Head::Record)` for a `Without`. A `Without` holds only records (part 3), so for every question of the form "may these two types have values in common", a `Without` has the head `Record`. `route`, the `_ <: Diff` arm, `diff_in`, `diff_left` and `known::disjoint` use `value_head()` and not `head()` (part 4.4 and 4.5). Without this, a `Without` would count as disjoint from no type and would pass every `- {..}`: the rule 11 check of a type test `x: { a: Int }` builds `{ a: Int } | r - { a: Int }`, and a `Without` would go to `r - { a: Int }` and from there to `r`, so a record that is not in `{ a: Int }` would take the clause.
- A helper `Ty::is_spread()`: true for a `Without`, and for an `Inter` with a `Without` member. This is the type of a written spread `{ ..R, … }` whose labels are not all known, and the "spread member" of a union (part 4.5). Every value of it is a record. Arm 5a, step a of arm 6, `route` and the type test of `pattern.rs` (part 14) use it.
- `Access::Label(Arc<str>)`: `v.f` on a record. `Access::Field` and `Access::Part` stay as they are.
- A helper `Ty::is_headed()`, true for `Con`, `Fn` and `Rec`. Every place that matches `Ty::Con(..) | Ty::Fn(_)` as "has a type constructor" is checked against it (part 16, step 4).
- `variances(Head::Record, _)` is never called, because `Rec` is not a `Con`. It gets an `unreachable!` arm with a comment.

`map_children`, `extrude`, `freshen`, `bound_vars`, `level_of`, `occurs` and the Coalescer walk every field of a `Rec` and the base of a `Without` with the polarity of the type around them.

**Types that have themselves inside them (§4.7).** A field of a `Rec` and the base of a `Without` count as places inside a type constructor, the same as the arguments of a `Con`: `bound_vars` (`solve.rs:1207`) passes `con = true` for them, and the Coalescer (`display.rs:~560`) goes in with `depth + 1`. So a cycle through a record field or a spread base is the existing error E0505, "this type has itself inside it, through a type variable". i has no recursive structural types: a recursive alias is already rejected, so a record type can never contain itself, and inference may not make one either. Two programs show what this means:

- `last = (r) { r.next |> { Nil -> r.v  n -> last(n) } }` walks a chain of records. Rule 13b gives `R <: { next: A }`, and the recursive call gives `A <: R | Nil`. The cycle goes through the field `next`, so it is E0505, with or without annotations. A chain, a tree or any other recursive data is a custom type (`Node(A) { Node(v: A, next: Node(A) | Nil) }`), and then `.next` uses rule 13a. The spec says so in §4.9.3.
- `build = (r, n) { n |> { 0 -> r  _ -> build(record.put(r, "a", n), n - 1) } }` gives `R :> Without(R, {a}) & { a: N }`: the cycle goes through the base of a `Without`, so it is E0505 too. The way out is a written parameter type that is not a type variable, `build = (r: {..}, n: Int) -> {..} { … }`. Then `record.put(r, "a", n)` has, in the solver, the type `Without(R2, {a}) & { a: Int }` with put's own `R2 :> {..}`. E4 does not apply to it, because its base is a variable. But it is checked against the written `{..}` of `r`, which is not a variable, so no variable gets a bound that holds itself, and there is no cycle. (In a shown type, `R2` is replaced by its lower bound `{..}`, and then E4 makes it `{ a: Int }`.)
- **The same cycle in a fold.** A spread or a `put` on the accumulator of a callback with no written type gives the same cycle: in `list.fold(xs, { n: 0, total: 0 }, (acc, x) { { ..acc, n: acc.n + 1 } })`, `acc` has a fresh `P`, fold's `B <: P` comes from the expected type `(B, A) -> B`, and the body gives `B :> Without(P, {n}) & { n: N }`. So `P :> B :> Without(P, {n}) & …`, and it is E0505. `record.put(acc, "n", acc.n + 1)` is the same. Two ways out: write the type of the accumulator, `(acc: { n: Int, total: Int }, x) { { ..acc, n: acc.n + 1 } }`, so that the spread is `{ n: Int, total: Int }` by E4; or build the new value as a full literal, `{ n: acc.n + 1, total: acc.total }`, as with a tuple `(acc.0 + 1, acc.1)`. So a spread and a literal that make the same value can get different verdicts here. That is the price of the simple rule.
- **A loop that keeps a generic rest.** A function with a label parameter that sets a field in a loop and keeps the other fields as a type variable `R` cannot be written: in `fill(R, K) = (r: R & {..}, key: K, n: Int) -> { ..R, K: Int } { n |> { 0 -> record.put(r, key, 0)  _ -> fill(record.put(r, key, n), key, n - 1) } }`, the call of itself gives `R` the lower bound `Without(R2, {K}) & { K: Int }` with `R2 :> R & {..}`, the same cycle. A written type that is not a variable (`r: {..}`, `-> {..}`) removes the cycle, and with it every other field of the result. Code that needs this builds the record without a loop, or uses a custom type.

Why not accept such a cycle? Its least solution is finite: when every field label of `F` is in `L` (always so for a spread or a `put`), `μX. I | Without(X, L) & F` is `I | Without(I, L) & F`, by E3 and because `Without` is monotone. But using it needs a new case in the occurs check, in `bound_vars` and in the Coalescer, which finds such cycles and rewrites them. That is more machinery than the design wants now; a later track may add it, and no program accepted now would change its meaning. The hint of E0505 says what to write when the cycle goes through a `Without`: "write the type of this parameter, such as `(acc: { n: Int, total: Int }, x)`, or build the new record as a full literal".

This keeps every shown scheme finite, so the read-back rule of 4.7 holds. Tests: `recursive_record.i`, `accumulate_put.i`, `fold_spread.i` and `fill_generic.i` in the reject list of 15.1, and `accumulate_put_annotated.i` and `fold_spread_annotated.i` in the pass list.

Field types and labels are kept sorted, so structural `==`, the constraint cache and the quick exit `l == r` (`solve.rs:246`) keep working.

### 4.3 Building types: only exact rewrites

Two smart constructors build every `Rec` and `Without`. They are the only place where these forms are rewritten, and they are used by `solve.rs`, by `Norm` and the Coalescer in `display.rs`, by known types, and after a label parameter is replaced.

`rec(fields)`:

- **N1.** Sort the fields. A label given twice has its two field types put together by `&`. This happens only when two `Param`s are replaced by the same `Name`, or in `&` (4.5). A written type or a literal with a label twice is a compile error before this point (E0540).

`without(T, L)`:

- **E0.** `L` is empty. The result is `T & {..}`.
- **E1.** `T` has no records: a `Con`, an `Fn`, `Never`, or a `Rec` with a field whose type is plainly empty. The result is `Never`. A type is **plainly empty** when it is `Never`, or a `Rec` or a tuple with a field or part that is plainly empty. (A type variable is not plainly empty.)
- **E2.** `without(A | B, L)` is `without(A, L) | without(B, L)`.
- **E3.** `without(Without(T, L1), L2)` is `without(T, L1 ∪ L2)`.
- **E4.** `without(Rec(fs), L)`, when no label of `fs` may be equal to a label of `L`: `Rec(fs` without the fields whose labels are the same as a label of `L)`.
- **E5.** `without(T & {..}, L)` is `without(T, L)`: a `Without` counts only the records of its base, so the member `{..}` adds nothing. This is what makes `put` on `r: R & {..}` show as `{ ..R, K: V }` and not `{ ..R & {..}, K: V }`.
- `without(Top, L)` is `{..}`.
- In every other case the form stays: `Without(T, L)`. This is the case for a type variable, an intersection, a difference, and a `Rec` with a label that may be equal to a label of `L`.

Each rewrite is an equality, so it may be used on either side of `<:`, and it stays true when every `Param` is replaced by any `Name`:

- E0 to E3 and E5 are equalities of sets, because a `Without` holds only records.
- E4 is an equality of sets too: the records that agree with a record of `{ fs }` at every label outside `L` are exactly the records of `{ fs }` without the fields at labels of `L`. This needs `{ fs }` to have values. When a field type of `fs` is plainly empty, E1 comes first and gives `Never`, which is the set rule of part 3. So `{ ..{ a: Never, b: Int }, -a }`, `{ ..{ a: Never, b: String }, -a }` and `{ ..Never, -a }` are all `Never`: one operator on equal sets gives one result. A field type with a type variable is not plainly empty, so E4 applies to it; a variable stands for the types that reach it, and when none reach it, no record of `{ fs }` is ever made, as for `(A, Int)` today.
- E4 needs that no label may be equal to a label of `L`. With `Param` labels, the set depends on the strings the `Param`s become, so the form stays and the rules of 4.4 work on it.

So rewriting a scheme in `display::simplify`, in either polarity, never makes a wrong program correct. Where it rewrites a `Without` on the right of `<:`, it can only make a correct program pass that R-without-right would reject.

No rewrite makes a type larger. In particular, `Without(a & B, L)` is not split, and `Without(Rec(fs), {K})` with a `Param` `K` is not turned into a smaller record.

### 4.4 Subtyping: the arms of `constrain_in`

The arms go into `constrain_in` (`solve.rs:245`) in this order. Arms that exist today keep their places.

1. (exists) `l == r`, `Never <: _`, `_ <: Top`; the cache.
   **Member (new, general).** `l <: r` when `l` is an `Inter` and `r` is one of its members. This is exact (`l & m` is a subset of `m`). It comes before the variable arms, so that `F & { K: V } <: F` holds at once and does not give `F` a lower bound that holds `F` itself (part 5.9).
2. (exists) `Union <: _`; `_ <: Inter`.
3. (exists) the variable arms: `Var <: r` and `l <: Var`, with `extrude` when the levels need it. A `Without` may be stored as an upper or a lower bound of a variable. That is safe, because every bound is checked again by these same arms.
4. **R-split.** `l <: Rec(gs)` with two or more fields, when `l` is not a `Rec`: `l <: Rec([g])` for each field. This is exact, because `{ a: A, b: B }` is `{ a: A } & { b: B }`. It must come before the left-`Inter` arm, so that `a & { x: A } <: { x: A2, y: B }` is tried one field at a time. Each part is smaller, so this ends.
5. **R-without-right.** `l <: Without(U, L)`:
   - when `l` is `Without(B, L2)` with `L2 ⊆ L` (by "the same"): `B <: U`;
   - when `l` is an `Inter` with a member `Without(B, L2)`, `L2 ⊆ L`: `B <: U` for that member;
   - else: `l <: U` and `l <: {..}`.

   This is sound: `Without` is monotone in its base, `Without(B, L2)` is a subset of `Without(B, L)` when `L2 ⊆ L`, `l & m` is a subset of `m`, and the records of `U` are a subset of `Without(U, L)`. It may reject a correct program. That only happens with a spread written in an input place, such as a parameter type `{ ..R, a: Int }`.
5a. **R-spread-union.** `l <: U`, when `U` is a `Union` with at least one spread member (a `Without`, or an `Inter` with a `Without` member), and `l` is a `Without(B, L2)` or an `Inter` with a `Without(B, L2)` member. The candidates are the spread members of `U` (`is_spread()`, part 4.2) whose `Without(U', L')` part has `L2 ⊆ L'` (by "the same"). With exactly one candidate `S`, constrain `l <: S`. With two or more, fail with `FailKind::Ambiguous`, as `route` does with two type variables: the choice would depend on the order of the members. With none, go on with the arms below, so that `l` may still fit the `Rec` member. A union with a spread member is only in an output place (E0518, part 8.2), so this arm checks a result against a written result type, or a value against a lower bound that came from such a type.

   This is sound, because `S` is a part of `U`. It is needed because the arms below split `l` before they look at the union. Without it, `f(R) = (c: Bool, r: R & {..}) -> { ..R, a: Int } | { b: Int } { c |> { True -> record.put(r, "a", 1)  False -> { b: 2 } } }` would fail or go wrong: the `True` branch has the type `Without(R2, {a}) & { a: 1 }`; arm 7 would try `{ a: 1 }` against the union (routed to `{ b: Int }`, so `NoField`), then `Without(R2, {a})`, which arm 6 step a sends to the `Rec` member `{ b: Int }`, and step f would give `R2 <: { b: Int }`. That try has no failure, so `f` would be accepted with the wrong upper bound `R <: { b: Int }`, and `f(True, { x: 1 })` would be E0512 at the call. With arm 5a, `l <: { ..R, a: Int }` is split by arm 2: the `Without` part gives `R2 <: R` by arm 5, and `{ a: 1 } <: { a: Int }` by arm 7. The candidates are found from the labels alone, so no bound is added before one is chosen. Test: `union_spread_written.i` in 15.1, the type of `union_spread.i` written as the result type.
6. **R-without-left.** `Without(B, L) <: U`, when `U` is a `Rec`, a `Con`, an `Fn`, `Never`, a `Union` or a `Diff`. (`U` is not a variable, an `Inter` or a `Without` here, because arms 2, 3 and 5 came first. A union with a spread member whose labels fit was taken by arm 5a. `Both` goes to its existing arm.) A `Without` holds only records, so it is treated as a type with the head `Record` (its `value_head()`, part 4.2). The steps, in order:
   - **a. A union.** Flatten `U`. If it has a member `R` that is a `Rec`, go on with `U := R` (after §4.4 merging there is at most one `Rec`). Else, if it has exactly one spread member (`is_spread()`), go on with that member (by arm 5, or arm 2 for an `Inter`); with two or more, fail with `FailKind::Ambiguous`. Else take its var-like members that are not records, as `route` does (`solve.rs:516`): with one, go on with `U :=` that member; with none, fail with `FailKind::Mismatch`; with more, fail with `FailKind::Ambiguous`. So a `Without` is never sent past a record member of a union to a variable, the same as a `Rec` is not.
   - **b. A plain variable.** If the member chosen in step a is a `Var`, constrain `Without(B, L) <: v` by arm 3. That is a stored bound, checked again later.
   - **c. A difference.** If `U` is `Diff(A, H)`: when some member of `H` has the `value_head()` `Record` (a `Rec`, `{..}` or a `Without`), fail at once with `FailKind::Mismatch`: every value of `Without(B, L)` is a record, and `- H` takes all records out. Else `Without(B, L) <: A`. This is §4.5 rule 3 with a `Without` counted as a record. It never sends the same pair back to this arm: `A` is a part of `U`.
   - **d. A type with another head.** If `U` is a `Con`, an `Fn` or `Never`, fail at once with `FailKind::Mismatch`.
   - **e. A record.** `U` is a `Rec` `R`. If a label `f` of `R` is the same as a label of `L`, or may be equal to one, fail at once, here, with the reason: `FailKind::NoField(f)`, "the field `f` may be missing here" (E0512), when the labels are the same, and `FailKind::LabelMayBe(K, f)`, "the key `K` may be `f`; it is not known here" (E0544), when they may be equal.
   - **f.** Else `B <: R`.

   This arm ends: each step goes on with a part of `U`, or with `B`, which is a part of the left side. No step gives the same pair to this arm again, so no cache entry is needed (the cache at `solve.rs:248` only keeps pairs with a `Var`).

   The last case is exact: `R` asks nothing about the labels of `L`, so a record of `Without(B, L)` is in `R` exactly when the record of `B` it came from is. Steps c (a difference that takes records out), d (another head) and e (a forgotten field is needed) fail at once, and they assume that `B` has a record: then a record of `Without(B, L)` is in no type with another head, in no `A - H` whose `H` has records, and, since a forgotten field can be anything or missing, not in `R`. If `B` has no records, `Without(B, L)` has no values and the check could pass, so these cases may reject a program that uses such an empty type. They never accept a wrong one. Every `Without` that `put`, `remove` or a spread expression makes has as its base the type of a value that was checked to be a record (`r: R & {..}` in part 5.1, `te <: {..}` in part 7), so in practice the base has records.

   The check fails here and not later. An earlier draft turned the record member into `Never` and went on with `B <: Never`. When `B` is a variable, that is only stored as a bound, so the failure came later as a plain mismatch with `Never`, far from its cause, or not at all when `B` had no lower bound. Failing at once keeps the reason and the place.
7. (exists) the left-`Inter` arm (`solve.rs:285`). Its partition counts `Rec` as concrete, next to `Con` and `Fn`: `matches!(m, Ty::Con(..) | Ty::Fn(_) | Ty::Rec(_))`. A `Without` member is tried alone, like a variable. **Which failure is kept:** today, when every try fails, the failures of the last try are kept. Now the failures of the first try that failed with `NoField` or `LabelMayBe` are kept, if there is one, and else those of the last try. So in `Without(R2, {K}) & { K: "s" } <: { a: A }`, both tries fail with E0544, and E0544 is what the user sees.
8. (exists) `Diff <: _`, `Top <: _`, `Both`, `_ <: Union` (route), `_ <: Diff`. `route` and `_ <: Diff` compare `value_head()`, not `head()` (part 4.2). A `Without` on the left never reaches them, because arm 6 came first; the change is for a `Without` member on the right of `_ <: Union`, and so that no other path can treat a `Without` as disjoint from records.
9. **R-width.** `Rec(fs) <: Rec(gs)`: for each `(g, G)` in `gs`, find the field of `fs` whose label is the same as `g`, and constrain `F <: G`. If there is none, fail. The failure is `LabelMayBe`, E0544 ("the key `K` may be `g`"), when a label of `fs` may be equal to `g`, and `NoField`, E0512 ("the record has no field `g`"), when not. With `env` true, fail: records are never services.
10. (exists) `Con <: Con`, `Fn <: Fn`, and the catch-all mismatch. `Con <: Rec` and `Rec <: Con` fall to the catch-all. When the left is a custom type and the right is a record, the message adds the hint of part 11.

### 4.5 `&` and `|`

In `solve.rs` (`inter_in` :744, `union_in` :698, `merge` :801) and in `display.rs` (`Norm::inter` :772, `Norm::union_in` :698):

- `Rec(fs) & Rec(gs)` is `rec(fs ++ gs)`: every label of both; labels that are the same are merged by `&` (N1). Labels that may be equal stay two fields. This is exact for every replacement: when `K` becomes `"a"`, N1 merges `{ K: A, a: B }` into `{ a: A & B }`, which is the same set.
- A `Rec` and a member with another head give `Never`, as today. That is exact: no record is an `Int`.
- `Without` members are not merged with anything. They stay as members of the `Inter`.
- `Rec(fs) | Rec(gs)` is `Rec` of the labels that are the same in both, with their types merged by `|`. This is the MLstruct join, and it makes a larger type, as §4.4 already does for `List(Int) | List(String)`. In a written type the merged type is what the union means (§4.4: members with one type constructor "are put together into one"). So `{ a: Int, b: Int } | { a: String }` **is** `{ a: Int | String }`.
- A `Without` member of a union is not merged, with a `Rec` or with another `Without`. Its labels are not known, so no join of it with a record is a record type: in `{ ..R, a: 1 } | { b: 2 }` the common labels depend on `R`. So a union may hold one `Rec` member and any number of members that are a `Without`, or an `Inter` with a `Without` in it. This is the one exception to "no two members of a type have the same type constructor" (spec §4.4), and the spec text says so (§4.4 and §4.9.3, part 12). It is safe: every rule that relies on one member per head (`route`, known types, `-`, type tests) looks for the `Rec` member, and a spread member is only reached by a record when there is no `Rec` member and it is the only spread member, or through a variable (part 4.4, arm 6, step a), or by its labels (arm 5a). When a later rewrite turns a `Without` member into a `Rec` (E1 to E4), `union_in` merges it as usual.
- **Only in an output place.** A written type with a union that has a spread member, in a place that is not an output place, is E0518 (part 8.2). This is the rule §4.4 gives a union with two or more type variables, for the same reason: inference cannot choose between such members. A record on the left goes to one member by a fixed rule (`route` below) and never tries another, so in an input place a correct call could be rejected: with `h(R) = (p: { ..R, a: Int } | { b: Int }) -> Int { 0 }`, the call `h({ a: 1 })` would go to `{ b: Int }` and fail, although `{ a: 1 }` is in the first member when `R` is `{..}`. In an output place, the body's value is checked against the union, and a rejection there only rejects more. Inference makes such a union only from lower bounds, which are output places, so an inferred scheme (as in `union_spread.i`, 15.1) keeps it in an output place too, and reads back without E0518.
- So `{ ..R, a: 1 } | { b: 2 }` is shown as it is, and reads back as the same two members (4.7): a written union is turned into a `Ty` with the same merge rules (T3 makes sure that this path uses `union_in` and the smart constructors), which do not merge them either. Such a union may be written as the result type of the function that makes it: arm 5a of 4.4 sends the spread part of the body's type to the spread member.
- In an environment (`env` true) there are never records.

`route` (:516) compares `value_head()` on both sides (part 4.2). After merging, a union has at most one `Rec` member, and a `Rec` routes to it by head. When a union has a `Rec` member and spread members, a record on the left goes to the `Rec` member. When it has no `Rec` member, a record goes to the spread member if there is exactly one, and fails with `FailKind::Ambiguous` if there are two or more, as `route` does today with two var-like members. So the result never depends on the order of the members. `A - H` (`diff_in` :877, `diff_left` :544) takes records out by `value_head()`, so `Without(B, L) - {..}` is `Never`. `known::disjoint` says a record or a `Without` is disjoint from every other head and never from a record or a `Without`.

### 4.6 Bounds: `add_bound`, `reach`, `compare`

The code is at `solve.rs:385-490`.

- `reach` and the `new` list in `add_bound` collect `Rec` bounds like `Con` bounds: the arms `Ty::Con(..) | Ty::Fn(_)` at :397 and :438 become `is_headed()`. If one is missed, a record bound is dropped with no error, so this is the most important line of the audit.
- **Every record upper bound is stored**, as every other bound is (`bounds.push` at :408). Nothing is keyed by label.
- The rule "a new upper bound is compared only with the first upper bound of each head" (:390-393) stays. For two record upper bounds `compare` does nothing, because no field of a record is invariant: there is nothing to check. So no check is lost.
- A variable's record upper bounds together act as their `&`. Each lower bound is checked against each upper bound (the loops at :266-270 and :272-276). So with `x <: { a: X }` and then `x <: { a: Y }`, every lower bound `lb` of `x` gets `lb.a <: X` and `lb.a <: Y`, which is `lb.a <: X & Y`. A second bound on the same label that conflicts with a lower bound fails there.
- Two record **lower** bounds: `compare` joins the fields with the same label, as for a covariant argument of a `Con` today (:473-482).
- `needs_join(Rec(fs))` is true only when some field needs a join (:1267). So records of plain data make no join variables.

### 4.7 Display

- `ShownType::Record { base: Option<Box<ShownType>>, fields: Vec<(String, Option<ShownType>)> }` in `shown.rs`. A field with `None` is a forgotten label, shown `-label`.
- `{ name: String, age: Int }` is printed with the fields sorted, and `{..}` for no fields. A label that is not a `name` is printed as a string literal: `{ "first-name": String }`. A `Param` label is printed as its `UpName`.
- The text `{ ..B, fs, -m … }` means `Without(B, labels(fs) ∪ {m …}) & Rec(fs)` (part 3). So an `Inter` of `Without(B, L)` and `Rec(fs)` is printed `{ ..B, fs, -l … }` only when every label of `fs` is in `L`, with `-l` for each label of `L` that is not a label of `fs`. This is the case for every type that `put` and spreads make. When a label of `fs` is not in `L`, it is printed `{ ..B, -l … } & { fs }`, which reads back as the same type. A `Without(B, L)` alone is `{ ..B, -l … }`. A test checks that every shown type reads back as the same type (15.2).
- `display::tree` (:1176 and :1280) gets an explicit `Rec` arm before the arm that shows every other `Con` as a tuple.
- The Coalescer and `Norm` use only the smart constructors of 4.3 for these forms.

Expect changes in the `.out` files of schemes: `(u) { u.name }` used to be an error (E0506), and now it has a type.

### 4.8 Known types (`known.rs`)

- `members` (:21) treats a `Rec` as one member with head `Record`. An `Inter` gives its `Rec` member, as it gives its `Con` member today. A `Without(B, L)` gives the members of `B`, where a record member loses every field whose label is the same as, or may be equal to, a label of `L`. That is a larger type, which is safe for a known type.
- `match_k` (:322) gets a `Rec` arm: when `P` is a `Rec` and `K` has one `Rec` member, each field of `P` is matched with the field of `K` with the same label (a `Param` is the same only as itself). A field with no field of the same label is skipped. `match_k` never sees a label parameter of another binding: the known type of a callee with label parameters is built with the label map of L4 (part 5.2, L4 step 3), so the only `Param` labels in it are the caller's own.
- A record literal has the record of the known types of its fields, with `_` for a field that has none.
- A spread `{ ..e, f1: e1, … }` has the known type `without(ke, {f1, …}) & rec(fi: ki)`, built with the smart constructors, where `ke` is the known type of `e` (`Top` when `e` has none, so that `without` gives `{..}`) and `ki` is the known type of `ei` (`_` when it has none). So `b = { ..a, n: 1 }` with `a = { u: User("bo") }` has the known type `{ n: 1, u: User }`, and `b.u.name` uses rule 13a, as `record.put(a, "n", 1).u.name` does. `{ ..e }` alone has the known type `ke & {..}`, built the same way: the record members of `ke`, and `{..}` when `e` has no known type. So `{ ..x }` always has a known type, and `g = (x) { { ..x } == 1 }` is the §6.9 error, because a record is never an `Int`. Part 12 says the same.
- `ksubst` builds types with the smart constructors. With the known function type built under the label map of L4, the known type of `record.put({ age: 1 }, "age", "x")` is `{ age: "x" }`.
- A key parameter `key: K` has the known type `String`, as its type. Known types never hold a label parameter as a type. L4 finds a key parameter by its name, not by a known type (part 5.2).
- `members` returns a type variable as a member with no head, as today. Rule 13b counts such a member like `_` (part 6).
- **T1's `whole_lits` and `hidden_inputs`** (T1 decision 7, `known.rs`) get record arms. In `whole_lits`, every field of a `Rec` is a covariant place, as a tuple part is, so its `Int` and `String` members get all their values and a `Never` becomes `_`; the base of a `Without` is covariant too, and the result is rebuilt with the smart constructors. `hidden_inputs` walks the fields and the base with the polarity of the record. Without these arms a literal inside a record field would keep its one-value known type where T1 makes it whole for a tuple: in `list.fold(xs, { n: 0 }, (acc, x) { { acc.n == 3 } |> { True -> acc  False -> { n: acc.n + 1 } } })` (the braces are needed, because `|>` binds tighter than `==`), rule 13b would give `acc.n` the known type `0`, and `acc.n == 3` would be a false E0520. With the arm, `acc` has the known type `{ n: Int }`, the same as `(0, 0)` gives `(Int, Int)`. Test: `known_fold_record.i` in 15.1.

## 5. Keys: label parameters

### 5.1 The library types

```i
get(K, V): (r: { K: V }, key: K) -> V
put(R, K, V): (r: R & {..}, key: K, value: V) -> { ..R, K: V }
remove(R, K): (r: R & {..}, key: K) -> { ..R, -K }
```

### 5.2 The rules

- **L1. What it is.** A type variable declared on a binding or a declaration (`get(K, V) …`) that is used as a label in a record type anywhere in the binding is a **label parameter**. "Anywhere" means every type written in the binding where the variable is in scope: its own annotations (parameter types, the `->` part, the type of the binding), and also the types written in its value, such as the annotations of inner bindings and functions (syntax.md §4.7 lets them use the variable). So in `f(K) = (r: {..}, key: K) -> Int { h = (s: { K: Int }) { s } ⏎ 0 }`, `K` is a label parameter of `f`, and `{ K: Int }` in `h` is a record type with the label `Param(K)`. Every `UpName` label is a declared type variable of a binding around it, and so it is a label parameter of that binding (part 8.2). There is no third kind of label: `Label` has only `Name` and `Param`. This is found from the syntax alone, in `resolve.rs`. A declared variable that is not used as a label anywhere in the binding is an ordinary type variable, even when it is the whole type of a parameter: `id(A) = (x: A) { x }` must stay an ordinary generic function, and resolve cannot tell it from a key without inference. So a function that passes its key on writes `K` as a label at least once, most often in its `->` part: `twist(K) = (r: { a: Int }, k: K) -> { ..{ a: Int }, K: "s" } { … }`. When a key argument is a parameter whose written type is a declared variable that is not a label parameter, E0541 says this (part 11).
- **L2. Where it may be written.** As a label, anywhere in the written types of its binding. As a type, only as the whole written type of a **top-level** parameter of the binding's own function: a parameter of the function that is the binding's value (`f(K) = (r: { K: Int }, key: K) …`); the parameter of a clause block with one clause and no guard that is the binding's value (`f(K) = { key: K -> … }`), because syntax.md §6.12 makes it the same function as `(key: K) { … }` and syntax.md §7.1 makes its `x: T` a parameter annotation; or a top-level parameter of the written function type of a declaration (`get(K, V): (r: { K: V }, key: K) -> V`). Such a parameter is a **key parameter**. Every label parameter must be the type of at least one key parameter, so that a call can fix it. The binding must be generalized (its value is a direct value, §6.10). These uses are E0543:
  - `K` as the type of a parameter of an inner function (`h = (y: K) { … }`);
  - `K` as the type of a parameter inside a written function type (`f: (y: K) -> Int`), also at the top level of the binding's own type;
  - `K` in a pattern of a clause block with more than one clause or with a guard (there `x: K` is a type test), in a pattern of a binding statement, inside a pattern (`Some(x: K)`, `(x: K, y)`), as a result type (`-> K`), or inside another type (`List(K)`, `K | Int`);
  - a type parameter of a type definition or of an alias used as a label.
- **L3. Inside its own binding.** The label parameter is rigid: it is `Label::Param(k)`, one string that the caller picks.
  - It is never an inference variable, and it never takes part in a constraint as a type.
  - A key parameter `key: K` has the type `String` (T1's String with the value set "all"), so `key` may be used as any String: compared, printed, passed to `record.has`. Its known type is `String` too.
  - The only thing that ties the value of `key` to the label `K` is the check at each call (L4). The limits of L2 make sure that the body never supplies a value for a place of type `K`: every such place is a top-level parameter, and its value always comes from the caller.
  - No new type form is needed: T1's String needs no case for keys, and known types hold no `K`.
- **L4. At a call.** At a call `f(a1, …)` (also with `|>`), where the scheme of `f` has label parameters:
  1. the key arguments (those of the key parameters) are inferred first, in order;
  2. each label parameter `K` gets a label from its key argument:
     - when the argument is a name that refers to a key parameter `j: J` of a binding around the call, or to a local name bound directly to such a name, through any number of such names: `K := Param(J)`. A name **refers to a key parameter** when it is a name that the key parameter's pattern binds at its top (`key`, and also `k2` in the alias parameter `k2 = key: K`, spec §6.11), or a name bound directly to one: by a binding statement `k2 = key`, or by the plain name pattern of a clause block with one clause and no guard whose argument is such a name (`key |> { k -> … }`, which syntax.md §6.12 makes the same as `k = key`). Forms the syntax defines as the same binding get the same verdict. The name is looked up by resolution, not by known types, so nothing but the key itself can pass. `second(key, s)`, `string.trim(key)`, a clause with a type test or a guard, or a clause block that gives `key` as its value are not keys.
     - else, when the known type of the argument is a String type with exactly one value `"s"` (a T1 literal type): `K := Name("s")`;
     - anything else (no known type, `String`, a union `"a" | "b"`, a cofinite set, `_`): E0541. The call then gets fresh types and nothing more.
     When `K` is the type of more than one key parameter, they must all give the same label, or it is E0541.
  3. the scheme is instantiated with these labels (`freshen` takes a label map and rebuilds with the smart constructors). For `K := Name("s")`, the parameter type `K` becomes the one-value String type `"s"`, so the solver checks that the argument is `"s"`; the known type only chose the label. For `K := Param(J)`, the parameter type `K` becomes `String`, and the check is the name rule of step 2.
     **The known function type gets the same label map.** At 1865366, the known type `kf` of a callee name is built in `infer.rs:1087-1112` before any argument is looked at: `display::known(&s.ty, …)` for an external, `other_known` for an imported binding, and `self.kvars` (from `scheme_known`) for a binding of the module. `call()` (`infer.rs:1260-1345`) then uses it for the expected type of every argument (`ksubst(&ps[i], &sigma)`) and for the known type of the result (`call_known`). For a callee whose scheme has label parameters, T3 builds `kf` after step 2, from the scheme with the label map put in (`display::known`, `other_known` and `scheme_known` take the map, or `kf` is rebuilt with it through the smart constructors), and only then works out the expected types of the other arguments. So the expected types and known types of the caller never hold a `Param` of the callee: with `K := Name("a")` the known parameter type `Box({ K: Int })` is `Box({ a: Int })`, and with `K := Param(J)` it is `Box({ J: Int })`, a label of the caller's own binding. The key arguments of step 1 get the expected type `String` and no known type from `kf`. Without this, a correct caller would be rejected: with `apply(K) = (b: Box({ K: Int }), key: K, f: (Box({ K: Int })) -> Int) -> Int { f(b) }`, the call `apply(Box(item: { a: 1 }), "a", (x) { x.item.a })` would give `x` the known type `Box({ K: Int })` with apply's `K`, rule 13a would put that into the solver, and `{ a: Int } <: { K: Int }` would fail with E0544. A test checks this program (`key_known_fn.i`, 15.1).
  4. the call goes on as every call does (rule 5 of §4.8), with the other arguments.

  So the solver never gets a label that is an inference variable, and no type or known type of a caller holds a label parameter of another binding.
- **L5. Not a value.** A name bound by a binding or a declaration that declares label parameters (L1) may only be the callee of a call (`f(…)`, or the right side of `|>`). Any other use of the name is E0542: `list.map(xs, record.get)`, `g = f`, `apply(f, …)`, a tuple `(f, 1)`. The rule is syntactic and is checked in `resolve.rs`, where label parameters are found. It holds **inside the binding's own value as well as outside it**. Inside its own value the binding has no scheme yet (§4.8), and its key parameter has the type `String` there (L3), so its self-type is a plain function type with `key: String`. If `f` could be used there as a value, a copy of it could be called with any string:

  ```i
  f(K) = (r: { K: Int }, k: K, n: Int) -> Int {
    n |> {
      0 -> record.get(r, k)
      _ -> { g = f ⏎ g(r, "zzz", 0) }
    }
  }
  f({ a: 1 }, "a", 1)
  ```

  Here `g = f` is not a call of itself (L6), and `g` would get the self-type `({ K: Int }, String, Int) -> Int`. `g(r, "zzz", 0)` would check, and at run time `record.get(r, "zzz")` on `{ a: 1 }` would reach `unreachable` in `$record_get`. The same holds for `apply(f, r, "zzz", 0)` with `apply = (h, a, b, c) { h(a, b, c) }`. With L5 as a syntactic rule, `g = f` and `apply(f, …)` are E0542, and every use of `f` inside its value is a call, which L6 checks. This is `self_value.i` in 15.1.

  A name in a type counts as a use too. T2's type of a value (`{ x }` in a type) and an alias pattern (T2, S6) read the type of a name. When that name is bound by a binding with label parameters, as `apply = (f: { record.get }, r) { f(r, "a") }`, it is E0542: else get's `Param(K)` would be in the type of a binding that does not declare it, against part 4.1.
- **L2a. A key parameter in a value type.** In T2's type of a value, or in an alias pattern, a name that refers to a key parameter (by the name rule of L4 step 2) is E0543. Such a type would be `K` written in another way, and L2 is a syntactic check that only sees a written `K`. If `{ k }` meant the declared type `K`, then `g(K) = (r: { K: Int }, k: K, f: (y: { k }) -> Int) -> Int { f("zzz") }` would bring back the hole that L2 closes (part 5.5): in the body `k` is a `String`, so `f("zzz")` checks, while a caller with `K := "a"` checks its callback with `y: "a"`. If it meant `String`, it would be only a longer way to write `String`. The rule is the same whatever S6 decides a value type means (part 19). It is checked in `resolve.rs` with L2.
- **L6. Recursion.** Inside its own value a binding is not yet generalized (§4.8). A call of itself must give every label parameter its own key parameter by name (`K := Param(K)`). Otherwise it is E0541, with the note that a function with label parameters calls itself only with its own keys. A binding with label parameters may not be in a recursive group with other bindings (spec §4.7: the other names of a group are not general, so a call between them has no scheme for L4). That is E0543 on the label parameter, with a note that names the other binding of the group. The groups are known before inference (`order.rs`), so the check is made there.
- **L7. Principal types.** The labels come from names and known types, which are found before solving, like the constructors of rule 13a. They never come from solving, so solving does not depend on its own order. Every binding that is accepted has a principal type; programs whose keys are not known are rejected, not guessed. An annotation can always make the key known.
- **L8. No escape.** A type that holds `Param(K)` may not become a bound of a type variable from outside the binding that declares `K`: a variable of an outer parameter, of an outer binding that is not generalized, or of a name of the binding's group. Such a variable is not generalized with the binding, so `K` would stay in it after every call has replaced `K`, and would leak into the scheme of the outer binding, where nothing declares it. In `outer = (acc) { f(K) = (r: { K: Int }, key: K) { acc(r) } ⏎ f({ a: 1 }, "a") }`, `acc` would get the type `({ K: Int }) -> B`. That is E0545, "the label parameter `K` of `f` reaches `acc`, which is outside `f`", at the place where the constraint is made.
  - **How it is found.** Each `LabelParamId` keeps a level: the **raised** level of the binding that declares it, the level at which the binding's own fresh variables are made. For a local binding this is `s.level` after the `self.s.level += 1` at `infer.rs:2056` (the binding is direct, by L2, so the raise always happens), not the outer level where the name is bound; generalization (`infer.rs:2108`) takes the variables above the outer level. For a top-level binding it is the level at which its value is inferred. So every variable from outside the binding, such as `acc` in the example, is at a level strictly below the level of `K`. If the outer level were stored, `{ K: Int }` would have `level_of` equal to the level of `acc`, `constrain_in` would take the plain `add_bound` arm with no `extrude`, and `K` would leak into the scheme of `outer` with no error. `level_of` (`solve.rs:203`) counts a `Rec` or `Without` with a `Param(k)` label as being at least at the level of `k`. So a constraint between such a type and a variable of a lower level goes through `extrude` (the variable arms, part 4.4 arm 3). `extrude` (`solve.rs:577`) to a level below the level of `k`, on a type with the label `Param(k)`, fails with `FailKind::LabelEscapes(k)` instead of copying it, because no fresh variable can stand for a label.
  - This is the same check as "a rigid type variable escapes its scope" in other languages. It keeps the rule of 4.1 true: `Param` labels exist only inside their binding and its scheme. So every shown scheme reads back (4.7), and the argument of 5.5 ("inside the binding") covers every place a `Param` can be.
- **L9. Not called by the host.** A binding whose scheme has label parameters may not be an entry point (§10.2: a host export, a test, `main`). The host calls an entry point with no L4 check, and the host signature shows a key parameter only as `String` (L3). So in
  ```i
  pick(K) = (k1: K, k2: K) -> Int {
    r: { K: Int } = record.put({..}, k1, 1)
    record.get(r, k2)
  }
  ```
  listed in `[wasm] exports`, the body checks (both keys give `Param(K)`), the host signature is `(String, String) -> Int`, and `pick("a", "b")` would reach `unreachable` in `$record_get`. `entry_point` (`infer.rs:378`) looks at the scheme first: when it has label parameters, it is E0320, "`pick` has a label parameter, so the host cannot call it: the host gives its keys with no check (§10.3)", and no host type is worked out. This is the only way to call a binding without a call in i code: L5 rules out every other use of its name as a value.

### 5.3 Two label parameters with the same label

`K` and `J` are two `Param`s, so inside the binding `{ K: Int, J: String }` has two fields. When a call fixes both to `"a"`, N1 merges them: `{ a: Int & String }`. That is the right set, because the body was checked for every choice of `K` and `J`, also for `K` = `J`.

So a body that writes `K` and then `J` is checked against "they may be equal":

```i
both(K, J) = (r: {..}, k: K, j: J) -> { K: Int, J: String } {
  r |> record.put(k, 1) |> record.put(j, "s")
}
```

This is rejected with E0544, "the key `J` may be `K`". The error comes from R-without-left (part 4.4, arm 6) when the type of the body, `{ ..{ ..R1, K: Int }, J: "s" }`, is checked against the field `K` of the written result: the field `K` of the inner record was forgotten by the second `put`, because `J` may be `K`. Without the error, `both({..}, "a", "a").a + 1` would add 1 to `"s"`.

### 5.4 The cases the task names

- **Union keys** (`"a" | "b"`): E0541 for `get`, `put` and `remove`. A later extension may allow them for `get` only, as `r <: { a: V } & { b: V }`. It must never be allowed for `put`.
- **Plain `String` keys:** E0541. A dynamic key belongs in a `Dict`. `record.has` takes a plain String.
- **A key with no known type** (a parameter with no annotation): E0541. The hint says to declare a label parameter.
- **Duplicate keys on `put`:** the field is replaced. That is the defined behavior, and the spread type gives it: `put(put(e, "a", 1), "a", "s")` has `{ a: "s" }` (E3, E4).
- **Duplicate labels in a literal or a written type:** E0540. Write `{ a: Int & String }`.

### 5.5 Why it is sound

Inside the binding, a type with `Param` labels stands for every type you get by replacing the `Param`s with strings. Every rule must stay true for each such replacement:

- R-width finds a field only by "the same" label, never through "may be equal". After a replacement, the same field is still there.
- `&` keeps fields that may be equal as two fields. After a replacement N1 merges them by `&`, which is the same set.
- `|` keeps only labels that are the same in both. The join is made once, when the type is built: a written union of records means its join (§4.4), and the body and the scheme hold the joined `Rec`. A replacement changes that `Rec` and never joins it again, so every check sees the same type before and after. (Joining again after a replacement would give a smaller type, because more labels may then be common; on the right of `<:` that could fail a check that passed. In `f(K) = (r: {..}, key: K) -> Int { h = (s: { K: Int } | { a: Int }) { 0 } ⏎ h({ b: 1 }) }` the type of `s` is `{..}`, at every call of `f`.) The only union that a replacement can join again is one with a spread member, when E4 turns that member into a `Rec`. Such a union is only in an output place (4.5), where a larger type is safe.
- Forgetting (E4, R-without-left) treats every label that may be equal as forgotten. After a replacement fewer labels may be forgotten, so every check that passed before still passes.

These two directions do not contradict each other: for `<:` and `&`, treating `K` as different from `a` only makes types smaller or checks stricter; for forgetting, treating `K` as possibly `a` only forgets more. Both are true for every string `K` may be.

The rules above are about `K` as a **label**. `K` as a **type** needs its own argument, because a replacement changes it in the other direction: in the body a key parameter has the type `String`, and at a call with `K := "a"` its type is `"a"`, which is smaller. A smaller type in an input place is safe only when the value really comes from the caller and the caller is checked:

- By L2, `K` is a type only at a top-level parameter of the binding's own function. The value of such a parameter always comes from a call, never from the body. The body sees the larger type `String`, so it assumes less than is true.
- At a call with `K := Name("s")`, the solver checks the argument against `"s"`. So the value of the key is the string of the label.
- At a call with `K := Param(J)`, the argument is the outer key itself, by name (L4). So its value is the string of the outer label, which the outer call checked in the same way.
- No call comes from outside i code: the name is only a callee (L5), and the host never calls such a binding (L9).
- So in every run, the value of a key parameter is the string its label stands for, and a `$record_get` with that key finds the field the types promise.

Without L2, the body could supply the value. In `g(K) = (r: { K: Int }, k: K) -> Int { h = (y: K) { record.get(r, y) }  h("zzz") + 1 }`, `h` has the type `(y: String) -> Int` in the body, so `h("zzz")` checks, and the call `g({ a: 1, zzz: "s" }, "a")` would read the field `zzz` as an `Int`. The same happens with `f: (y: K) -> Int` in a written function type, where the body calls `f` and the caller's function is checked with `y: "a"`. L2 makes both E0543, and L2a does the same for `(y: { k }) -> Int`, where `k` is the key parameter. Without the name rule of L4, a known type could carry `K` to a value that is not the key, as in `record.get(r, second(key, s))` with `second(A) = (x: A, y: A) -> A { y }`; that is E0541.

### 5.6 Why `put` is not `R & { K: V }`

With that type, and `put` replacing the field at run time:

```i
bump(R) = (r: R) -> R & { age: String } { record.put(r, "age", "old") }
x = bump({ age: 36 })   // { age: 36 & String }, which has no values
x.age + 1               // checks, and at run time adds 1 to "old"
```

With `{ ..R, age: String }`, `x` has the type `{ age: String }`, and the last line is a compile error. A `put` that stops the program on a key that is already there would make `R & { K: V }` sound, but width subtyping hides fields: a `{ name, age }` value passed as `{ name }` would make code stop that looks correct.

### 5.7 The rigid key and forgetting

```i
twist(K) = (r: { a: Int }, k: K) -> { ..{ a: Int }, K: "s" } { record.put(r, k, "s") }
twist({ a: 1 }, "a").a + 1    // E0501: the type is { a: "s" }
twist({ a: 1 }, "b").a + 1    // correct: the type is { a: Int, b: "s" }
```

The `->` part is needed: it is what makes `K` a label parameter (L1). Without it, `K` is an ordinary type variable, and the call of `put` in the body is E0541 with the hint of part 11.

The result type of `twist` keeps `Without({ a: Int }, {K})` as a form (E4 does not apply, because `a` may be `K`), so each call works it out exactly after `K` is fixed. Inside the body, reading `.a` after the `put` is E0544, "the key `K` may be `a`":

```i
peek(K) = (r: { a: Int }, k: K) -> ({ K: "s" }, Int) {
  x = record.put(r, k, "s")
  (x, x.a)                    // E0544 at .a
}
```

The constraint is `Without(R2, {K}) & { K: "s" } <: { a: A }`, both members of the left `Inter` fail with `LabelMayBe` (the `Rec` by R-width, the `Without` at once by R-without-left, step e), and the left-`Inter` arm keeps that failure (part 4.4, arm 7). The same body with `r: {..}` instead of `r: { a: Int }` gives the same E0544 in the body, and not a mismatch at some later call.

### 5.8 A schema library in plain i

```i
Schema(A) { Schema(decode: (Json) -> A ? DecodeError) }

object: Schema({..}) = Schema(decode: (j) { {..} })

field(R, K, V) = (s: Schema(R), key: K, value: Schema(V)) -> Schema({ ..R, K: V }) {
  Schema(decode: (j) {
    s.decode(j) |> record.put(key, value.decode(json.at(j, key)))
  })
}

user = object |> field("name", string) |> field("age", int)
// user : Schema({ age: Int, name: String })
```

This checks as written, with no extra annotation:

- `Schema` is covariant in `A` (its parameter is only in an output place, `infer.rs:2239`).
- Declared type variables are flexible (syntax.md §4.7). Inside `field`, `s.decode(j)` has the type `R`. The call of `put` adds `R <: R2 & {..}` for put's own fresh `R2`, so `R` gets the upper bound `{..}`. The shown scheme is `field: (s: Schema(R & {..}), key: K, value: Schema(V)) -> Schema({ ..R, K: V })`.
- In the call of `put`, the key argument is the name `key`, the key parameter of `field`, so put's label parameter becomes `Param(K)` (L4).
- The body's type `Schema({ ..R2, K: V2 })` is checked against the written `Schema({ ..R, K: V })`. The right side is split; R-without-right takes the `Without` member and gives `R2 <: R`; the `{ K: V2 }` member gives `V2 <: V`.
- At `object |> field("name", string)`: `K := "name"`, `R` gets `{..}`, and the result is `without({..}, {name}) & { name: String }`, which E4 makes `{ name: String }`. The second call gives `{ age: Int, name: String }`.

A test checks this exact program (part 15).

If a later track makes declared type variables rigid, `field` would write `s: Schema(R & {..})`. That is an open question for D-types (part 19).

### 5.9 A codec in both directions

The json-schema track needs a codec that decodes and encodes. With one type parameter, `Schema(A) { Schema(decode: (Json) -> A ? DecodeError, encode: (A) -> Json) }`, the encode half of `field` cannot be typed. It gets `x : { ..R, K: V }` and must pass it to `s.encode : (R) -> Json`, which needs `Without(R, {K}) & { K: V } <: R`. That is false when `R` may already have a field `K` of another type, and this design has no type for "`R` has no field `K`". Taking the field out with `record.remove` does not help: `Without(…, {K})` is still larger than `R`.

A codec with two type parameters works, with `&` and no forgetting on the encode side. `E` is the type the encoder reads from, and `D` is the type the decoder makes:

```i
Codec(E, D) { Codec(decode: (Json) -> D ? DecodeError, encode: (E) -> Json) }

object: Codec({..}, {..}) = Codec(decode: (j) { {..} }, encode: (x) { json.object([]) })

field(E, R, K, V) = (s: Codec(E, R), key: K, value: Codec(V, V)) -> Codec(E & { K: V }, { ..R, K: V }) {
  Codec(
    decode: (j) { s.decode(j) |> record.put(key, value.decode(json.at(j, key))) },
    encode: (x) { json.set(s.encode(x), key, value.encode(record.get(x, key))) },
  )
}

user = object |> field("name", string) |> field("age", int)
// user is shown as Codec(Never, { age: Int, name: String }); see below
```

- `Codec` is contravariant in `E` and covariant in `D` (`infer.rs:2239`).
- In `encode`, `x` gets no useful expected type. By §4.8, an argument of a call gets the type of the callee's parameter, here `(E0) -> Json` from the constructor `Codec`, with `E0` filled only from the other arguments, never from the `->` part of `field`. So `x` has a fresh type `X`. `s.encode(x)` gives `X <: E`, and `record.get(x, key)` gives `X <: { K: V3 }` (the key is the name `key`, so L4 gives `Param(K)`), and `value.encode(…)` gives `V3 <: V`.
- The body's type is then checked against the written result. `Codec` is contravariant in its first parameter, so this needs `E & { K: V } <: X`, which goes on to the upper bounds of `X`: `E & { K: V } <: E`, which the Member arm (part 4.4, arm 1) makes true at once, with no bound that holds `E` itself, and `E & { K: V } <: { K: V3 }`, which the left-`Inter` arm gives by R-width (`V <: V3`). With `V3 <: V`, the encode side checks.
- **What `user` is, and what is shown.** `user` is not generalized: its value is a call, not a direct value (§6.10), so its type variables stay those of the module. On the encode side they get only **upper** bounds, because `Codec` is contravariant in `E`: `object` gives `E1 <: {..}`, the first `field` gives `E2 <: E1 & { name: String }`, and the encode type of `user` is `E2 & { age: Int }`. No type comes **to** `E1` or `E2`. So the encode side accepts exactly the records of `{ age: Int, name: String }`: `user.encode(v)` gives `v <: E2` and `v <: { age: Int }`, and the new lower bound `v` of `E2` is checked against `E1 & { name: String }` and `{..}`. But a variable that is not generalized is shown as the union of the types that come to it (spec §4.7), which is `Never` here, so `user` is shown as `Codec(Never, { age: Int, name: String })`. The same holds for what an alias pattern reads: `Codec(User, _) = user` would bind `User` to `Never`. The decode side is covariant and has the lower bound `{ age: Int, name: String }`, so the type of the records is read from there: `Codec(_, User) = user` binds `User` to `{ age: Int, name: String }`. When the shown encode type matters, write the type of `user`: `user: Codec({ age: Int, name: String }, { age: Int, name: String }) = …`. The json-schema track (plan, T2/S6 alias pattern) must read the decode side; part 19 asks it.
- A key given twice gives `E & { a: Int } & { a: String }`, which is `{ a: Int & String }`: no value can be encoded, and the decode side has `{ a: String }`. That is safe; a library may check the keys when it builds the codec.

So the bidirectional builder can be written in plain i with this design, with two type parameters. A one-parameter codec would need a new form, "a record type with no field `K`" (a "lacks" constraint). That is not in this design; part 19 asks D-types and json-schema whether it is wanted. A test checks the program above (part 15).

### 5.10 Typed macros (M4)

M4 lands after T3 and expands typed macros (meta.md §7) after their call is checked. L1, L2, L2a, L5 and L6 are checked from the syntax in `resolve.rs`, before inference, and 5.5 depends on L2 for every place where `K` is a type. So M4 must keep these rules for the code a macro gives back. This is a requirement on M4, sent with G3 (part 13); T3 makes it easy to meet:

- **The checks run again on the expansion.** T3 writes the checks of L1, L2, L2a, L5 and L6 as one function over the HIR of a binding (`resolve.rs`), not only as part of the first pass, so that M4 can run it on the expanded code with the label parameters of the bindings around the call. In the program of 5.5, a typed macro `m` in `g(K) = (r: { K: Int, zzz: String }, k: K) -> Int { m(r) }` can take the label `K` out of `type_of(r)` and give back `{ h = (y: K) { record.get(r, y) } ⏎ h("zzz") + 1 }`. The check of L2 on the expansion makes that E0543, as it is when the same code is written by hand.
- **No new label parameters.** The label parameters of a binding are fixed before inference, from its written types. An expansion may use a label parameter of a binding around the call as a label, but it may not use a declared type variable of such a binding as a label when it is not already a label parameter: in `f(A) = (r: {..}, key: A) { m2(r) }`, an expansion with `{ A: Int }` in it is E0543, "`A` is not a label parameter of `f`; write it as a label in a type of `f`". Inference has already treated `A` as an ordinary type variable there.
- **The `Types` service (meta.md §7.4).** `type_of` shows a type with the text of 4.7: a `Param` label as its `UpName`, a `Without` as a spread. `subtype` and `members` read that text back with the same rules. `subtype(a, b)` answers `No` only when neither type has a type variable **or a label parameter** in it; a label parameter is a string that is not known, so `subtype({ K: Int }, { a: Int })` is `NotKnown` (it is true when `K` is `"a"`). `members` gives a record type as one member, and a spread member of a union as its own member (4.5).
- **A label parameter in a record type is a name, for hygiene (meta.md §5).** meta.md §5 says that labels are compared by their text only. That stays true for `name` and string labels. But an `UpName` label in a record type is a reference to a declared type variable (L1, part 8.2), so it is looked up as a name, with its scope (text and marks), as the same `K` written as a type (`y: K`) is. In `f(K) = (r: {..}, key: K) -> { ..{..}, K: Int } { m(r) }`, a syntax macro `m` whose template writes the type `{ K: Int }` has its own marked `K`, which is not the caller's label parameter: it is a type variable that nothing declares, a compile error, and not a capture of `f`'s `K`. Only an `UpName` that came from the caller's code (no mark of the run) is `f`'s `K`. This applies to M2 (syntax macros) as well as M4.
- **L4 and L9 need nothing new.** A call in the expansion is checked by inference like any other call, so L4 fixes its labels. An expansion cannot make a binding into an entry point.

## 6. Field access `v.f`

`field()` (`infer.rs:1538`) works out the known type `kv` of `v` first, as today. Then:

- **13a. Fields of constructors.** When `v` is known to be made by one constructor (§7.5), or every member of `kv` is a custom type, also when `kv` has no members (`Never`): the existing code, unchanged. It records `Access::Field`, and the type of `v` must be a subtype of its known type.
- **13b. Fields of records.** When `v` has no known type, or `kv` has at least one member and every member is a record type, `_` or a type variable (a declared variable such as `A` in `(x: A) { x.name }`, or a variable from an expected type): make a fresh `A`, constrain `t <: { f: A }`, give `v.f` the type `A`, and record `Access::Label(f)`. The known type of `v.f` is the field `f` of the one `Rec` member of `kv`, when it has one.
- **13c. Tuple parts.** `v.k`: unchanged.
- **Otherwise** (a custom type and a record, or a function, an `Int`, …): E0512 with the message "`.f` reads a field of a record or of a custom type, and this value may be either; write its type". The code generator never sees a mixed target, so the shortcut "the last branch is not tested" in `Lowerer::field` (`lower.rs:1675`) stays correct.

E0506 is no longer given for `.f` (it is still given at `infer.rs:1270`). Every program that is correct today has a known type at each `.f` whose members are all custom types, or that has no members, so it keeps rule 13a and its code. The two rules never overlap: 13a needs every member to be a custom type, and 13b needs no known type or at least one member. So `list.map([], (x) { x.name })`, where `x` has the known type `Never`, keeps `Access::Field([])` and the type `Never`, as today.

A written type only adds information (§4.7), so it never turns a correct `.f` into an error: `(x) { x.name }`, `id_name(A) = (x: A) { x.name }` and `names(A) = (xs: List(A)) { list.map(xs, (x) { x.name }) }` all use 13b. The known type of `v.f` under 13b is what lets `r.user.name` use 13a when `r = { user: User("bo") }`, and lets `r.cb(x: 1)` call by label when `r.cb` is a function.

A custom value that reaches a record function is a mismatch with a hint:

```i
User { User(name: String) }
name_of = (u) { u.name }        // (u: { name: A }) -> A
name_of(User("bo"))             // E0501
```

> `User` is a custom type, not a record. `.name` here reads a record field because `u` has no known type (§4.8 rule 13b). Write `(u: User) { u.name }` to read the field of the constructor.

## 7. Record expressions and type tests

- **Literal** `{ f1: e1, …, fn: en }`: the parts are worked out left to right (§6.2). Each part has the expected type of the same field of the one `Rec` member of the expected type, if there is one. The type is `rec(fi: ti)`. A label twice is E0540.
- **Spread** `{ ..e, f1: e1, … }`: `e` first, then the parts in order. `e` gets no expected type. Each `ei` gets the expected type of the field `fi` of the one `Rec` member of the expected type of the whole spread, if there is one, as in a literal. So with `Cfg = { cb: (User) -> String, n: Int }`, in `next: Cfg = { ..base, cb: (u) { u.name } }` the parameter `u` has the known type `User`, and `.name` uses rule 13a. `te <: {..}`, and the type is `without(te, {f1, …}) & rec(fi: ti)`. `{ ..e }` alone has the type `te & {..}`. A label twice among the parts is E0540, as in a literal. The known type is in part 4.8.
- **Same label.** Two labels are the same when their strings are the same, after the escapes of a string literal are worked out. A `name` label is the string of the name. So `{ a: 1, "a": 2 }`, `{ a: 1, "\u{61}": 2 }` and `{ ..r, a: 1, a: 2 }` are E0540. HIR keeps each label as its string (`LabelName` holds the string with no escapes), and E0540, the sort, `Ty::Rec` and the label constants of part 9 all use that string. The same holds in a written type: `{ a: Int, "a": String }` and `{ ..R, a: Int, -a }` are E0540.
- **Empty record** `{..}`: the type `{..}`.
- **Effects and direct values:** `has_no_effect` (`infer.rs:2340`) treats a literal as it treats a tuple: a literal whose parts give the warning gives the warning "its value is not used" as a statement that is not the last. A spread is treated as a constructor update `C(..v, …)` (the spec's "record update", renamed in part 12), which gives no warning. Part 12 adds this to the list of §6.10. A literal of direct values is a direct value (§6.10) and is generalized (rule 14).
- **Type tests.** `x: { name: String }` in a clause with more than one clause takes every record, by head. Rule 11 already asks for `B <: T | r - T`, so every record that can come to that place must be in `{ name: String }`. To tell kinds of records apart, use a custom type. `lower.rs` `head_cond` (:2209) maps `Head::Record` to a test of tag 14.
- **No record patterns** in T3. `{ name: n } = r` is a parse error with the hint "read the field: `n = r.name`".

## 8. Syntax

### 8.1 Expressions

A `{` that begins an expression, and the `{` of a function body, are read like this. The parser finds the first part with one scan: it ends at the first `,` or break that is directly inside these braces, or at the matching `}` (found with `close[]`). "Top level" means not inside `( )`, `[ ]`, `{ }` or `<< >>`.

1. `{}` is the empty clause block (syntax.md §6.12).
2. `{..}`, and a `{` followed by `..`, is a record.
3. A `{` whose first part begins with a `name`, an `UpName` or a string literal and then `:`, and either has no top-level `=`, `->`, `?` or `|`, or ends with a `,` (not a break), is a record. An `UpName` label is then the compile error "a label is a name or a string", also when it is the first field: `{ Type: 1, b: 2 }` gives that message, not a parse error of a block. (No block or clause block starts with `UpName :` without one of these tokens either; the test of the `,` claim below covers this too.)
4. Otherwise, the rule of syntax.md §6.1: a clause block or a block. For a function body, it is always a block.

So:

| Source | Reading |
|---|---|
| `{ x: Int -> x }` | clause block (it has `->`) |
| `{ x: Int = 1 ⏎ x }` | block (it has `=`) |
| `{ name: "ada", age: 36 }` | record |
| `{ ..r, age: 37 }` | record |
| `{ "first-name": "ada" }` | record |
| `{ Type: 1, b: 2 }` | record, with the error "a label is a name or a string" |
| `{ f: (x) -> Int { x }, n: 0 }` | record (the first part ends with `,`) |
| `{ f: (x) -> Int { x }, }` | record with one field |
| `{ f: (x) -> Int { x } }` | clause block (it has `->` and no `,`) |
| `{..}` | the empty record |
| `(n) { name: n }` | a function that gives a record |
| `(x) { x: Int = 1 ⏎ x }` | a function whose body is a block |
| `(x) { x: Int -> x }` | a function whose body is a block; that block is a compile error, as today |

- `||` and `|>` are other tokens, so they do not count as `|`.
- **Why a `,` decides.** A block or a clause block never has a `,` directly inside its braces: statements and clauses are ended by breaks, and on one line by the end of the longest expression (syntax.md §1.7). So a first part that ends with `,` can only be a record field. (T3 checks this claim against the A1 parser with a test that no correct program of the test suite has such a `,`.)
- **The one misreading.** A record whose first field holds a function with a `->` part, and whose first field is not followed by a `,` on the same line, has a top-level `->` in its first part, so it is read as a clause block. This is `{ f: (x) -> Int { x } }`, or a record over several lines whose first line is such a field. The way out is a `,` after the first field. The error of a clause block whose head is `name: (…)` says this.
- **The printer.** `format.rs` and `print.rs` write a `,` after the first field whenever its value has a top-level `=`, `->`, `?` or `|`, also in the form with one field per line. So every record has a normal-surface form that reads back as the same record, and `i fmt --to=i` can print every record of the Lisp surface (part 13).
- Inside a record, a break is a `,`, as inside `( )`. A `,` at the end is allowed. But a break does not decide rule 3; only a written `,` does.
- Labels in expressions are `name`s or string literals. An `UpName` label is a compile error ("a label is a name or a string"), in the first field (rule 3) as in any other. A string literal used as a label must not have an interpolation (`\(x)`, track STR): a label is fixed when the program is compiled (labels are sorted then, E0540 compares them, and `Ty::Rec` holds them). An interpolated string as a label is a compile error, "a label is a string with no `\( )`". Raw and multi-line string literals with no interpolation are allowed.
- `.f` reaches only labels that are `name`s. Other labels are read with `record.get`.
- `C(..v, f: e)` stays the update of a custom value.

Why a function body uses the same scan instead of a forced error: a block never starts with `label :` without `=`, `->`, `?` or `|`, so no block is lost, and `(n) { name: n }` is the natural way to write a function that gives a record. A forced error would only send the user to `(n) { { name: n } }`.

### 8.2 Types

```
Prim       = FnType | TupleType | NamedType | "_" | "{" Type "}" | RecordType .
RecordType = "{" ".." "}"
           | "{" ".." Type { "," RPart } [ "," ] "}"
           | "{" FPart { "," FPart } [ "," ] "}" .
RPart      = Label ":" Type | "-" Label .
FPart      = Label ":" Type .
Label      = name | string_lit | UpName .
```

- A `{` in a type is a record type when `..` follows it, or a label and then `:`. Otherwise it groups a type, as today.
- `{ ..R }` alone is allowed and is `R & {..}`. It is the written form of the type of the expression `{ ..e }`.
- **Places (§4.4, E0518).** The §4.4 rule "an intersection with a type variable only in an input place" counts members that are type variables. In `{ ..R, f: T }`, which is `Without(R, {f}) & { f: T }`, no member is a type variable: `Without(R, {f})` is a form with `R` in a covariant place inside it, as `List(R)` is. So a spread with at least one field or `-` part may be in any place, and `put(R, K, V): (…) -> { ..R, K: V }` is correct. `{ ..R }` alone has no such form: it is `R & {..}`, an intersection with the type variable `R`, so in an output place it is E0518, the same as `R & {..}` written out. Two spellings of one type always get the same verdict.
- **A union with a spread member (§4.4, E0518).** A union that keeps a spread member (part 4.5: a spread whose labels are not all known, after aliases are expanded and the members are merged) may only be in an output place, as a union with two or more type variables. So `h(R) = (p: { ..R, a: Int } | { b: Int }) -> Int { 0 }` is E0518, and `f(R) = (c: Bool, r: R & {..}) -> { ..R, a: Int } | { b: Int } { … }` is correct.
- `check_places` (`infer.rs:603`) gets a `TypeKind::Record` arm. It visits every field type with the polarity of the record (fields are output places, as in a tuple), and the base of a spread with the same polarity (the base is covariant). A spread with no fields and no `-` parts whose base is a type variable (after aliases are expanded) is E0518 when the polarity is not `Neg`. Its union arm counts a spread member like a type variable member for the rule above: E0518 when the polarity is not `Pos`. So `bad(A) = (x: A) -> { a: A & Int } { … }` is E0518, because the field `a` is an output place. `walk_variance` (`infer.rs:2269`) gets the same arm: field types and the base of a spread are covariant, so a type parameter used only in record fields of a custom type's fields goes in the same direction as the type.
- A `string_lit` label has no interpolation (8.1).
- `-label` parts are allowed only after a `..` part (the grammar says so: `FPart` has no `-`).
- An `UpName` label must be a declared type variable, which makes it a label parameter (L1). A label that starts with a capital letter is written as a string: `{ "Type": String }`.
- Inside a record type, a break is a `,`. `TypeGroupBreak` (`parser.rs:1263`) applies only to groups.
- `{ x }` (T2's type of a value, in a group) and `{ x: T }` (a record type) differ only by the `:`.
- In a function's `->` part, a `{` right after `->` is part of the type (syntax.md §1.7), so `(x) -> { name: String } { name: x }` is a function with a record type and a record body.
- `Person = { name: String }` is an alias. `Person { name: String }` stays a parse error, with the hint "a record type is written `Person = { name: String }`".

This answers meta.md §11 question 2 for records: a `{ }` with one part in a type is a group, unless it is `{..}`, begins with `..`, or is `label: T`.

### 8.3 AST, HIR and the printer

- `ast::ExprKind::Record { base: Option<Box<Expr>>, fields: Vec<RecordField> }`, with `RecordField { label: LabelName, label_span: Span, value: Expr }` and `LabelName = Name(String) | Str(String)`.
- `ast::TypeKind::Record { base: Option<Box<TypeExpr>>, parts: Vec<RecordTypePart> }`, with each part a field or a forgotten label.
- The same nodes in HIR, with type labels resolved to `Name(String)` or `Param(TyVarId)`.
- `format.rs` and `print.rs`: a record on one line when it fits, `{ name: "ada", age: 36 }`, and else one field per line with no commas. The formatter keeps "the same tree after formatting".

## 9. Run time (WasmGC)

### 9.1 The value

- An `$obj` with field 0 (tag) = 14, field 1 = 0, field 2 = 0, and field 5 = a `$values` array `[l0, v0, l1, v1, …]`.
- Each `li` is a String value. The labels are sorted by `$string_compare` (`values/strings.rs:137`): bytes, then length.
- Field 5 is never null. The empty record has an empty array, so that two empty records compare equal.
- The tag table in `runtime/mod.rs:37-52` gets the row `| 14 | Record | field 5: labels and values, interleaved, sorted by the bytes of the labels |`.

### 9.2 `==`, compare and hash

They need no new code. The generic `$obj` path of `$value_compare_i32` (`values.rs:125`) compares the tag first, then fields 2 and 1, then field 5 element by element. Its hash (`values.rs:269`) mixes fields 1 and 2, the length and every element. So:

- `{ a: 1, b: 2 } == { b: 2, a: 1 }`, because both are sorted;
- `{ a: 1 } != { a: 1, b: 2 }`, by length, even when both have the type `{ a: Int }`;
- `{ a: 1 } != { b: 1 }`, by the label;
- a record is never equal to a tuple with the same array, because the tags differ;
- `hash` agrees with `==`, and does not depend on how the record was built, on the order of first use, or on `-O0` and `-O2`.

A record that holds a function compares it by identity, as a tuple does.

### 9.3 Runtime functions

In a new file `runtime/values/record.rs`, included from `values.rs`:

| Function | Does |
|---|---|
| `$record_new(values) -> eqref` | Takes an interleaved array that lowering has already sorted. |
| `$record_get(r, label) -> eqref` | Binary search. At each probe it tries `ref.eq` on the label first, then `$string_compare`. A missing label traps with `unreachable`; the types make that impossible (with L5 and L9, every key comes from a checked call in i code), and tests check it. |
| `$record_put(r, label, v) -> eqref` | Copies the array, and puts the field in or replaces it. |
| `$record_with(r, pairs) -> eqref` | A sorted merge of `r` and an already sorted interleaved array, for `{ ..r, … }`: one copy. |
| `$record_remove(r, label) -> eqref` | Copies the array without the field (the same record when it is not there). |
| `$record_has(r, label) -> eqref` | Whether the field is there, as a Bool value (`$bool_new`, `runtime/values.rs:40`). |
| `$record_keys(r) -> eqref` | The labels as a `List(String)`, in order. |
| `$record_size(r) -> eqref` | The number of fields, as an Int value (`$int_from_i64`, `runtime/bigint.rs:87`). |
| `$record_texts(r) -> eqref` | A `List((String, String))`: each label, in order, with the text form of its value (the code of `inspect`, 9.5). |
| `$is_record(v) -> i32` | Tag 14, in `emit/mod.rs` next to `$is_ctor` (:16). Used only by `Cond::IsRec`, never as a value. |

Every function that an `ilang:record` external maps to returns `eqref`, because `Op::Runtime` is emitted as `(call $name args)` and its result is used as an atom (`emit/mod.rs:539`). So `has` and `size` make a Bool and an Int value inside the runtime function; the `i32` and `i64` forms are local to `record.rs`. A test validates a module that calls each of the seven functions.

**Label constants.** Lowering makes one immutable global String for each distinct label used in a literal, a spread, a `.f` or a key argument (below) in the output module. The String builder of 1865366, `string()` in `emit/mod.rs:458-476`, emits `(call $string_from_bytes (array.new_data $bytes $sN …))`. Neither a call nor `array.new_data` is a constant expression, so it cannot initialize an immutable global. A label global is written out instead, with the layout of `$string_from_bytes` (`runtime/values/strings.rs:8`) and only constant instructions. Every label global is named by a number, `$label_N`, never by the text of its label: a label is any string, and a space, `(`, `)`, `"` or a byte that is not ASCII is not allowed in a WAT name, so `{ "a b": 1 }` would make text that does not assemble. The number is kept in a table keyed by the bytes of the label, as `string()` already keys its `$sN` data segments. Here is the global of the label `name`:

```wat
(global $label_3 (ref $obj)
  (struct.new $obj (i32.const 3) (i32.const 0) (i64.const 0) (f64.const 0)
    (array.new_fixed $bytes 4 (i32.const 0x6e) (i32.const 0x61) (i32.const 0x6d) (i32.const 0x65))
    (ref.null $values) (ref.null $limbs) (ref.null $call)))
```

`struct.new`, `array.new_fixed`, `ref.null` and `i32.const` are constant expressions in WasmGC. A test checks that `$string_from_bytes` and this form give the same fields (if T1 or STR changes the String layout, the label global follows it), and that the module validates.

**The limit of `array.new_fixed`.** Engines limit its number of operands: V8 takes at most 10000. `string()` uses `array.new_data` and has no such limit. So emit uses a constant `FIXED_MAX = 10000` and two fallbacks:

- A label with more than `FIXED_MAX` bytes gets a mutable global instead, `(global $label_N (mut eqref) (ref.null eq))`, numbered in the same table, set from `string()` at the beginning of `$start` (`emit/mod.rs:86`), the code that sets the program's `$g` globals today, before any of them. Its uses are `(ref.as_non_null (global.get $label_N))`. It is still one object, so `ref.eq` works.
- A literal or spread part list with more than `FIXED_MAX / 2` fields is built with `(array.new $values (ref.null eq) (i32.const 2n))` and one `array.set` for each label and value, in a local, and then passed to `$record_new` or `$record_with`. The order is the same, so the record is the same.

Tests: a record with a label of 10001 bytes, and a literal with 5001 fields, validate and run, and give the same `==`, `hash` and `inspect` as the same records built with `put`.

**Key arguments.** The key argument of `record.get`, `put`, `remove`, or of any function with label parameters, is an ordinary String expression, and `string()` builds a new String object each time it runs. So the labels of every record built with `put` would be fresh objects, and `ref.eq` would never match them; that is the schema-library case the fast path is for. So lowering passes the label global for a key argument when L4 fixed its label to `Name("s")`. Lowering cannot redo L4, which uses known types, so the checker hands the result over in a new table of `Typed` (`types/mod.rs`, next to `access` and `type_tests`):

```rust
/// For each call of a binding with label parameters: the index of each key
/// argument (after `|>` and labels are put in order) and the label L4 fixed for it.
pub key_labels: HashMap<(ModuleId, Span), Vec<(u32, KeyLabel)>>,
pub enum KeyLabel { Name(Arc<str>), Param }
```

`call()` fills it after L4 step 2, keyed by the span of the call, as `access` is keyed by the span of `.f`. (M2 changes the tables keyed by `(ModuleId, Span)` to expression ids; this table follows them.) For `KeyLabel::Name("s")`: a string literal is replaced by the label global of `"s"`; any other expression is worked out as written, for its effects, and its value is dropped, and the label global of `"s"` is passed. The argument lists of calls and operators are atoms (`Value::Call { args: Vec<Atom> }`, `Value::Op(op, Vec<Atom>)`), and at 1865366 no atom names a label global (`Atom::Global` is the `$g{N}` slot of a top-level value), so part 9.4 adds the atom `Atom::Label(LabelConst)`. This is correct, because the solver checked that the argument has the one-value type `"s"`, and Strings compare by their bytes, so no program can tell the two values apart. For `KeyLabel::Param`, the argument is the outer key itself, by name (L4), and is passed as written, so it is the label global that the outermost call passed. A call with no entry in the table passes its arguments as written. `record.has` takes a plain `String` and is not changed. Codegen tests check that `{..} |> record.put("name", "ada")` and `k = "name"` then `{..} |> record.put(k, "ada")` both store the label global of `name`, and that `.name` on the result hits `ref.eq`.

### 9.4 IR and lowering

The wasm crate already uses "Record" for constructors with fields (`CtorRepr::Record`, `TypeRepr::Record`), so the new names use `Rec`:

- `Atom::Label(LabelConst)`: a label global, as an argument of any call or operator, for the key arguments of 9.3. `emit::atom()` gets an arm that writes `(global.get $label_N)`, or `(ref.as_non_null (global.get $label_N))` for a long label (9.3). `LabelConst` is the number of the label in the table of 9.3.
- `Value::RecNew(Vec<(LabelConst, Atom)>)`: the atoms are worked out in source order first (§6.2), then put in label order, which is fixed when the program is compiled. Emitted as `(call $record_new (array.new_fixed $values 2n …))`, or with `array.new` and `array.set` above `FIXED_MAX / 2` fields (9.3).
- `Value::RecGet(Atom, LabelConst)`: `ExprKind::Field` with `Access::Label`. `Access::Field` and `Access::Part` are unchanged.
- `Value::RecWith(Atom, Vec<(LabelConst, Atom)>)`: a spread.
- `Cond::IsRec(Atom)`: `head_cond` (`lower.rs:2209`) for `Head::Record`, in T1's type-test region.
- The `"ilang:record"` externals: a new arm in `external_op` (`lower.rs:1364`, next to `"ilang:core"` at :1380) that maps each name to its runtime function, as `CORE`/`CORE_FUNCS` do, as `Op::Runtime`. Each of these functions returns `eqref` (9.3).

### 9.5 inspect

`runtime/values/inspect.rs` gets a branch for tag 14. Task kinds 0 to 3 are all in use (value, byte, parts, list rest; `inspect.rs:291-311`), and the packing `kind | index << 2` (:244) stays as it is:

- kind 0 on a record: an empty array writes `{..}`. Else write `{ `, push the byte `}` and the byte ` ` (kind 1), then push `(2, record, 0)`.
- kind 2 whose value has tag 14 (the parent's tag decides): `index` is always even. Write `, ` when `index > 0`, then the label and `: `, then push `(2, record, index + 2)` and `(0, value at index + 1, 0)`.
- A label that is a `name` (§1.3) is written as it is. Any other label is written in the String text form with `$ib_quote`. A small runtime function `$label_is_name` checks this.

The text form is `{ age: 36, name: "ada" }`, and `{..}` for the empty record. Both read back as the same record.

### 9.6 The host

Records are not in §10.3, so the existing checks reject them at the host boundary (E0318 for `@external`, E0320 for entry points and services). T3 adds a test. `abi.rs` needs no change.

That check looks only at the host signature, and a record that stays inside the function does not show there. So `entry_point` also rejects a binding whose scheme has label parameters, with E0320 (L9). A declaration with `@external` from the host and label parameters is already E0318, because by L1 its written type has a record type.

### 9.7 Cost

A `.f` on a record is a binary search with `ref.eq` and string compares: slower than `struct.get` on a custom value, and fine for records of the size of a schema or a JSON object. A `put` copies the array, so building n fields one `put` at a time costs O(n²). `docs/performance.md` gets a note and a micro-benchmark. Shape ids and inline caches can come later on the same layout, as long as `==` and `hash` stay canonical.

## 10. The module `i/record`

`lib/i/record.i`, registered in `crates/ilang-package/src/library.rs` (the list at :12), like `i/core`:

```i
//// Records (§4.9): reading and setting fields by a key that is one known string.

(get, put, remove, has, keys, size, texts)

/// The field `key` of `r`.
@external("ilang:record", "get")
get(K, V): (r: { K: V }, key: K) -> V

/// `r` with the field `key` set to `value`: added, or put in place of the field that is there.
@external("ilang:record", "put")
put(R, K, V): (r: R & {..}, key: K, value: V) -> { ..R, K: V }

/// `r` without the field `key`, if it has one.
@external("ilang:record", "remove")
remove(R, K): (r: R & {..}, key: K) -> { ..R, -K }

/// Whether `r` has a field `key`. It may have more fields than its type shows.
@external("ilang:record", "has")
has: (r: {..}, key: String) -> Bool

/// The labels of `r`, in the order of their bytes.
@external("ilang:record", "keys")
keys: (r: {..}) -> List(String)

/// The number of fields of `r`.
@external("ilang:record", "size")
size: (r: {..}) -> Int

/// Each label of `r`, in the order of its bytes, with the text form of its value (§11.8).
@external("ilang:record", "texts")
texts: (r: {..}) -> List((String, String))
```

- `r |> record.put("age", 37)` works because `|>` is a call.
- `record.get(r, "name")` is the same as `r.name`, and also reads labels that are not names.
- There is no `record.empty`: the empty record is `{..}`.
- **Reading every field.** There is no function that gives the values of a record as values, such as `entries: (r: {..}) -> List((String, A))`. The fields of a record have different types, and i has no type for "a value of one of these types" that a caller could use: with a type variable `A`, the function would claim that every value is an `A` for every `A`, which is false. Code that needs each value with its own type knows the labels, and uses `.f`, `record.get` or a typed builder (the `Codec` of part 5.9; a JSON encoder is built that way). Code that only needs to show the fields uses `texts`, which gives the text form of each value (§11.8). That is what a log line (`log.info("login", { user: "ada", tries: 3 })` writes `tries=3 user="ada"`), a failed property or test, and a debug print need, and it is plain i on top of `texts`.
- `has` and `keys` take plain Strings and give plain Strings. They do not change any type. `has(r, key)` with `key: K` is allowed, because `key` is a String (L3).
- An `update` is plain i:

```i
update(R, K, A, B) = (r: R & { K: A }, key: K, f: (A) -> B) -> { ..R, K: B } {
  record.put(r, key, f(record.get(r, key)))
}
```

## 11. Errors and hints

T3 takes the block **E0540–E0549**, whatever order T1, T2 and T3 merge in. T1 and T2 number their new codes from E0523 upward and stay below E0540.

| Code | Name | When | Message and hint |
|---|---|---|---|
| E0540 | `DUPLICATE_LABEL` | a label twice in a literal or a written record type | "the label `a` is given more than once"; for a type: "write `{ a: Int & String }`" |
| E0541 | `LABEL_NOT_KNOWN` | a key argument that is not the name of an outer key parameter and whose known type is not one string; a call of itself with another key | "the key of `record.put` must be one known string, such as `\"age\"`; here it may be any `String`" (or "… may be `\"a\"` or `\"b\"`"). Hint: "declare a label parameter, and use it as a label in a type of the function: `f(R, K, V) = (r: R, key: K, value: V) -> { ..R, K: V } …`; or write the key as a string, such as `record.get(r, \"a\")`". A clause that matches the key does not help: in `s |> { \"a\" -> record.get(r, s) … }`, `s` is still a `String` (§7.5 narrows only to constructors), and the hint does not suggest it. When the key argument is a parameter whose written type is a declared variable that is not a label parameter (L1), the hint says: "`K` is not a label parameter, because no record type of `f` uses it as a label; write it as a label, for example in the `->` part" |
| E0542 | `LABEL_PARAM_VALUE` | a name bound by a binding with label parameters used other than as a callee, inside the binding's own value or outside it, also in T2's type of a value or an alias pattern (L5) | "`record.get` has a label parameter, so it may only be called: write `(r) { record.get(r, \"name\") }`" |
| E0543 | `LABEL_PARAM_PLACE` | a label parameter as a type anywhere but the whole type of a top-level parameter of its binding's own function (an inner function, a parameter inside a written function type, a pattern, a result, inside another type); one that is no parameter's whole type; a name that refers to a key parameter in T2's type of a value or in an alias pattern (L2a); on a binding that is not generalized; on a binding in a recursive group with other bindings; a type parameter used as a label; a declared type variable that is not a label parameter used as a label in the code a typed macro gives back (5.10) | "the label parameter `K` may only be a label, or the type of a parameter of this function"; for a group: "a function with label parameters may not be recursive with `g`" |
| E0544 | `LABEL_MAY_BE` | a check that needs a rigid label to differ from another label (`FailKind::LabelMayBe`, from R-width or R-without-left) | "the key `K` may be `name`; it is not known here" |
| E0545 | `LABEL_PARAM_ESCAPES` | a type with a label parameter that would become a bound of a type variable from outside its binding (L8, `FailKind::LabelEscapes`) | "the label parameter `K` of `f` reaches `acc`, which is outside `f`". Hint: "pass the field's value out of `f`, not the record with the key `K`" |
| E0518 | (exists, `TYPE_PLACE`) | a union with a spread member (4.5) in a place that is not an output place; `{ ..R }` alone in a place that is not an input place (8.2) | for the union: "a union with a spread type as a member may only be in an output place (§4.4): inference could not choose the member for a value that comes here" |
| E0505 | (exists) | a type that has itself inside it through a record field or the base of a spread (part 4.2) | the existing message; when the cycle goes through the base of a spread (a `Without`), the hint: "write the type of this parameter, such as `(acc: { n: Int, total: Int }, x)`, or build the new record as a full literal" |
| E0512 | (exists, `NO_FIELD`) | a missing field of a record (`FailKind::NoField`, from R-width); a field that a `put` or a spread may have forgotten (R-without-left, step e) | "the record has no field `b`"; "the field `b` may be missing here". This is the code that `field()` already gives when a constructor has no field `f` (`infer.rs:1631`), so reading a field that is not there has one code for custom values and for records |
| E0501 | (exists) | a custom value given to a record type; a field value of the wrong type | "`User` is a custom type, not a record. … Write `(u: User) { u.name }` to read the field of the constructor." |
| parse | (exists) | an interpolated string as a label | "a label is a string with no `\( )`" |
| E0512 | (exists) | `.f` on a value whose known type mixes custom types and other types | "`.name` reads a field of a record or of a custom type, and this value may be either; write its type" |
| E0318, E0320 | (exist) | a record at the host boundary; E0320 also for an entry point whose binding has label parameters (L9) | the existing messages; for L9: "`pick` has a label parameter, so the host cannot call it: the host gives its keys with no check (§10.3)" |
| parse | (exists) | `Person { name: String }` | hint "a record type is written `Person = { name: String }`" |
| parse | (exists) | `{ name: n } = r` | hint "there are no record patterns; read the field: `n = r.name`" |
| E0501 | (exists) | `{}` where a record is needed | hint "`{}` is the empty clause block; the empty record is `{..}`" |

An example of a union key, in the new syntax:

```i
key = flag |> { True -> "a"  False -> "b" }   // known type "a" | "b"
record.get(r, key)                            // E0541
```

There is no error code for a spread with a type variable in an input place. Such a spread is checked with the sound rule R-without-right, which may only reject programs, so no extra rule is needed.

## 12. Spec text, section by section

Nothing is renumbered. New text goes into new subsections. The text is written for the spec after A1, T1 and T2, and is put there after both merged.

**§1.7 Lines.** In the list of what a break ends, add: "Directly inside the `{ }` of a record (§6.13) or a record type (§3), a break is a `,`, as inside `( )`."

**§3 Type syntax.** Add `| RecordType` to `Prim`, and the `RecordType`, `RPart` and `Label` lines of part 8.2 of this design. Add these bullets:

- "`{ name: String, age: Int }` is a record type, and `{..}` is the type of all records (§4.9). A label is a `name`, a string, or a label parameter (§4.9.4). A label that starts with a capital letter is written as a string."
- "A `{` is a record type when `..` follows it, or a label and then `:`. Any other `{ }` groups a type. So `{ x }` is a group and `{ x: T }` is a record type."
- "`{ ..R, f: T }` is `R` with the field `f` set to `T`, and `{ ..R, -f }` is `R` with the field `f` forgotten (§4.9.2). `{ ..R }` is the records of `R`."
- "A string used as a label may not have an interpolation."
- "Two labels are the same when their strings are the same, so `a`, `"a"` and `"\u{61}"` are one label. A label may not be in a record type twice, also not as a field and a `-` part."

**§4.1.** Add: "Records (§4.9) are values of one type constructor."

**§4.4.** At the end of the bullet on members with one type constructor, add: "All records count as one type constructor (§4.9.3). A spread type `{ ..R, … }` whose labels are not all known (its base is not a record type, or one of its labels may be a label parameter) is not put together with a record type; it stays a member of its own (§4.9.3)." In the bullet on members, "So no two members of a type have the same type constructor" becomes "So no two members of a type have the same type constructor, except that a union may have spread types next to one record type (§4.9.3)." In the bullet on places, "a union with two or more type variables as members may only be in an output place" becomes "a union with two or more type variables as members, or with a spread type as a member (§4.9.3), may only be in an output place". After "an intersection with a type variable only in an input place", add: "Here a member counts when it is a type variable. `{ ..R, f: T }` and `{ ..R, -f }` have no such member (§4.9.2), so they may be in any place; `{ ..R }` alone is `R & {..}`, so it may only be in an input place when `R` is a type variable." At the end of that bullet, add: "The fields of a record type are output places."

**§4.5.** Add after rule 7: "8. Records: see §4.9.3."

**§4.8, rule 12** (narrowing): after "each with new type variables as its arguments", add: "A taken record type is `{..}`: all records are one type constructor, with no arguments. So in `x |> { y: { a: Int } -> y.a  n -> n + 1 }` with `x: Int | { a: Int }`, `n` has the type `Int`."

**§4.8, rule 13** is replaced by:

> 13. **Fields and tuple parts.**
>   - (a) **Fields of constructors.** For `v.f`, when `v` is known to be made by one constructor (§7.5), or `v` has a known type and every member of it is a custom type (also when it has no members), every constructor that may have made `v` (§6.7) must have a field with the label `f`. The type of `v.f` is the union of the types of these fields, with the type arguments put in, and the type of `v` must be a subtype of its known type.
>   - (b) **Fields of records.** When `v` has no known type, or its known type has at least one member and every member is a record type, `_` or a type variable: `v.f` has the type `a`, with a new type variable `a`, and the type of `v` must be a subtype of `{ f: a }`. When the known type of `v` has a member that is a record type with the field `f`, the known type of `v.f` is the type of that field.
>   - (c) **Tuple parts.** For `v.k`, every member of the known type of `v` must be a tuple with more than k parts. The type of `v.k` is the union of their parts k, and the type of `v` must be a subtype of its known type.
>
>   In every other case it is a compile error.

Add rule 15:

> 15. **Records.** If every `ei : Ti`, then `{ f1: e1, …, fn: en } : { f1: T1, …, fn: Tn }`. If `r : R`, `R <: {..}`, and every `ei : Ti`, then `{ ..r, f1: e1, … } : { ..R, f1: T1, … }`. `{..} : {..}`.

In the known types, add:

- "A record has the record of the known types of its fields, with `_` for a field that has none. `{ ..e, f1: e1, … }` has `{ ..K, f1: K1, … }`, where `K` is the known type of `e` (`{..}` when it has none) and `Ki` the known type of `ei` (`_` when it has none). So `{ ..e }` alone has `{ ..K }`, which is `{..}` when `e` has none."
- "`v.f` read by rule 13b has the known type of the field `f` of the record member of the known type of `v`, when there is one." (The existing bullet for `v.f` covers rule 13a and 13c only; under 13b the type of rule 13 is a new variable and would give nothing.)
- In **To match**: "When `P` is a record type and `K` has one member that is a record type, each field of `P` is matched with the field of `K` with the same label."
- In the call bullet: "When the type scheme of `f` has label parameters, they are fixed first (§4.9.4), and the known type of `f` is the one with these labels put in."
- In the expected types: "A field of a record, also a field written after `..e`, has the matching field of the expected type, when that type has one member, a record type with that label. The `e` of `..e` has no expected type."

The example line becomes: "So `(u) { u.name }` has the type `(u: { name: A }) -> A`, and may not be given a `User`. `(u: User) { u.name }` reads the field of the constructor, and with `opt: Option(User)`, `opt |> { Some(u) -> u.name  None -> "" }` is correct."

**§4.9 Records (new).**

> **4.9.1 Values.** A record is a value made of fields. Each field has a label, which is a String, and a value. No label is in a record twice. The order of the fields has no meaning: `{ name: "ada", age: 36 }` and `{ age: 36, name: "ada" }` are the same record.
>
> **4.9.2 Types.** `{ f1: T1, …, fn: Tn }` is the type of the records that have at least the fields `f1 … fn`, where the value of `fi` is in `Ti`. A record may have more fields than its type says. They are kept, and `==`, `core.inspect` and `core.hash` see them. `{ f1: T1, …, fn: Tn }` is the same as `{ f1: T1 } & … & { fn: Tn }`. `{..}` is the type of all records.
>
> `{ ..R, f1: T1, …, -g1, … }` is `R` with the labels `f1 …` and `g1 …` **forgotten**, and `{ f1: T1, … }`. `R` with the label `f` forgotten is the type of the records that have the same fields as a record of `R` at every other label; at `f` they may have any value, or no field. So `{ ..{ age: Int, name: String }, age: String }` is `{ age: String, name: String }`. When `R` is a record type whose labels are known and that has values, forgetting `f` drops its field. When `R` has no values, the spread has none: `{ ..{ a: Never, b: Int }, -a }` has no values, as `{ ..Never, -a }`. Only the records of `R` count, so `{ ..R }` is `R & {..}`, and `{ ..Int, a: Int }` has no values. A spread with a field or a `-` part is one form, in which `R` is in an output place, as in `List(R)`; it is not an intersection that has `R` as a member (§4.4).
>
> **4.9.3 Rules.** Records are one type constructor. So a record is never a value of a custom type, a tuple, a function type, or a built-in type, and the rules of §4.4 for one type constructor hold:
>
> - `{ fs } <: { gs }` when every label of `gs` is in `fs`, and the field type in `fs` is a subtype of the field type in `gs` (width).
> - `{ fs } & { gs }` has the labels of both. A label in both has the `&` of its two types.
> - `{ fs } | { gs }` is put together into the record of the labels in both, with the `|` of their types. So `{ a: Int, b: Int } | { a: String }` is `{ a: Int | String }`. To tell kinds of records apart, use a custom type.
> - A spread type `{ ..R, … }` whose labels are not all known is not put together with a record type or with another spread type. Its labels are not all known when its base `R` is not a record type (a type variable, or an intersection or difference with one), or when one of its labels may be a label parameter, as `{ ..{ a: Int }, K: String }`, where `K` may be `a`. So `{ ..R, a: Int } | { b: Int }` is a union of two members, and stays one. A union has at most one record type as a member, and may have spread types next to it. Such a union may only be in an output place (§4.4): a record that comes to it goes to the record type, or to the one spread type when there is no record type, and inference does not try the other members. When `R` is a record type and no label may be a label parameter, the spread is a record type, and the rule above puts it together.
> - In a type test or a clause, a record type that is taken (§4.8 rule 12) is `{..}`.
> - A record type is a type constructor for §4.7: a type may not have itself inside it through a field or through the base of a spread. So `last = (r) { r.next |> { Nil -> r.v  n -> last(n) } }` is a compile error. Recursive data, such as a chain or a tree, is a custom type: `Node(A) { Node(v: A, next: Node(A) | Nil) }`. A function that sets a field in a loop, such as `build = (r, n) { n |> { 0 -> r  _ -> build(record.put(r, "a", n), n - 1) } }`, needs a written type for its parameter: `build = (r: {..}, n: Int) -> {..} { … }`. The same holds for the accumulator of a fold whose callback uses a spread or `record.put` on it: `list.fold(xs, { n: 0, total: 0 }, (acc, x) { { ..acc, n: acc.n + 1 } })` is a compile error, and `(acc: { n: Int, total: Int }, x)` or the full literal `{ n: acc.n + 1, total: acc.total }` is correct. A loop cannot keep the other fields as a type variable: with a written type that is not a type variable, they are only the fields of that type.
> - A type test `x: { … }` in a clause takes every record (§4.8 rule 11). So `x: { a: Int }` on a value of type `{ a: Int } | { a: String }` is a compile error.
>
> Why setting a field forgets it first: if `record.put(r, "age", "old")` had the type `R & { age: String }`, then for `r = { age: 36 }` it would be `{ age: 36 & String }`, which has no values, and `.age + 1` would be correct and add 1 to `"old"`. With `{ ..R, age: String }` it is `{ age: String }`, and `.age + 1` is a compile error.
>
> **4.9.4 Label parameters.** A type variable declared on a binding or a declaration that is used as a label in a record type anywhere in the binding, also in a type written inside its value, as `K` in `get(K, V): (r: { K: V }, key: K) -> V`, is a **label parameter**. A label that starts with a capital letter is always a label parameter of a binding around it. A declared type variable that is not used as a label anywhere in its binding is not a label parameter, also when it is the whole type of a parameter. A label parameter may be used as a label. As a type, it may only be the whole type of a parameter of the binding's own function, or of a top-level parameter of the written function type of a declaration (as `key: K` in `get` above). It may not be the type of a parameter of an inner function, of a parameter of a function type inside another type, or of a parameter of a function type written for a binding that has a value, and it may not be in another pattern, a result or another type. When the binding's value is a clause block with one clause and no guard, its parameter counts as a parameter of the function (§6.12). Such a parameter is a **key parameter**, and every label parameter must be the type of some key parameter. The binding must be generalized (§4.8 rule 14), it may not be in a recursive group with other bindings (§4.7), and it may not be an entry point (§10.2), because the host gives its keys with no check. A type with the label parameter may not become part of the type of a name from outside the binding, such as a parameter of an outer function; that is a compile error. A type of a value (§3) or an alias pattern may not name a key parameter, or a name bound by a binding with label parameters.
>
> At each call of a name whose type has label parameters, each label parameter takes a label from its key argument. When the argument is the name of a key parameter of a binding around the call (also the other name of an alias parameter `k2 = key: K`, §6.11), or a name bound directly to one (by `k2 = key`, or by the name pattern of a clause block with one clause and no guard, `key |> { k2 -> … }`), it takes that outer label parameter. Otherwise the known type of the argument must be a String type with exactly one value, and the label parameter takes that string; the argument must then have that one-value type. In every other case it is a compile error. So `record.get(r, "name")` is correct, and `record.get(r, s)` with `s: String` is not. A name bound by a binding with label parameters may only be called, also inside the binding's own value: it may not be used as a value, given to another name, or passed to a function. Inside its own value, a binding calls itself only with its own key parameters.
>
> Inside its binding, a label parameter `K` is one string that is not known. A key parameter `key: K` there has the type `String`. `{ K: A }` and `{ f: B }` have different fields, and `{ ..R, K: V }` forgets every field of `R` that may be `K`. Where a check needs to know that `K` is not `f`, it is a compile error: "the key `K` may be `f`".
>
> **4.9.5 The module `i/record`.** See §14.

**§6.1 Grammar.** In the `Function` line, the body `Block` becomes `( Block | Record )`, as part 8.1 reads the `{` of a function body. Add `| Record` to `Primary`, with:

```
Record      = "{" ".." "}"
            | "{" ".." Expr { "," RField } [ "," ] "}"
            | "{" RField { "," RField } [ "," ] "}" .
RField      = ( name | string_lit ) ":" Expr .
```

Add: "A string used as a label may not have an interpolation."

Replace the bullet "A `{` followed by a label `name :` … is kept for records" by the reading rule of part 8.1: "A `{` that begins an expression or a function body is a record when it is `{..}`, when `..` follows it, or when its first part begins with a `name` or a string and then `:`, and either has no `=`, `->`, `?` or `|` outside brackets, or is followed by a `,`. The first part ends at the first `,` or break directly inside the braces. A function body that is not a record is a block. So `{ f: (x) -> Int { x } }` is read as a clause block; write `{ f: (x) -> Int { x }, }`."

**§6.2.** Add: "In a record `{ f1: e1, … }` the parts are done in the order written. In `{ ..e, f1: e1, … }`, `e` is done first."

**§6.8.** "The body is a block, and the value of the block is the value that the function gives back" becomes "The body is a block or a record (§6.13), and its value is the value that the function gives back."

**§6.10.** In the list of direct values, add "a record whose parts are direct values". In the list of expressions that give the warning "its value is not used" ("literals, names, functions, and tuples, lists, constructor calls, `.f`, …"), add "records with no `..`" after "lists". A record with `..e`, like `C(..v, …)`, does not give the warning.

**§6.13 Records (new).**

> `{ f1: e1, …, fn: en }` makes a record (§4.9). A label may not be given twice, also not after `..e`; `a` and `"a"` are the same label. `{ ..e, f1: e1, … }` makes the record `e` with the given fields added or put in place of its fields; `e` must be a record. `{..}` is the record with no fields; `{}` is the clause block with no clauses (§6.12).
>
> `v.f` reads the field `f` (§4.8 rule 13). A field whose label is not a `name` is read with `record.get`. Fields are set and removed with `record.put` and `record.remove` (§14):
>
> ```i
> ada = { name: "ada", age: 36 }
> older = { ..ada, age: ada.age + 1 }
> r = {..} |> record.put("name", "ada") |> record.put("age", 36)
> ```

**§6.7.** Replace "If not, every member of the known type of `v` (§4.8) must be a custom type, and they are all the constructors of these types." by "If not, and every member of the known type of `v` (§4.8) is a custom type, they are all the constructors of these types. When `v` has no known type, or every member of it is a record type, `_` or a type variable, `v.f` reads a field of a record (§6.13). In every other case `v.f` is a compile error." Add at the end of the paragraph: "See §4.8 rule 13."

**§7.1.** Add: "A type test whose type is a record type takes every record (§4.9.3). There are no record patterns."

**§7.4.** Add: "All records count as one type constructor."

**§9.2.** "A service is a value of a named type, normally a record of functions" becomes "A service is a value of a named type, normally a custom type whose constructor holds functions. A record (§4.9) is not a named type, so a record type, also through an alias such as `Clock = { now: () -> Int }`, is never a service type." The E0225 message of `env_type` (`resolve.rs:1445`) becomes "a service type is one named type `T(…)`: not a type variable, a union, a record type, or `Never` (§9.2)". `env_type` keeps sending `TypeKind::Record` to `_ => None`, and a test (`record_service.i`, 15.1) guards it.

**"Record update" for custom values.** The spec calls `C(..v, f: e)` a "record update" (the grammar name `RecordUpdate` in §6.1, the known types of §4.8, §6.2, and any other place the spec after A1 has it). With structural records in §4.9 and `{ ..e, … }` in §6.13, that name is confusing. It becomes "constructor update" (`CtorUpdate` in the grammar) everywhere, with no change of meaning. The word "record" in the spec then means only the values of §4.9.

**§10.3.** Add: "Records may not go to the host. A binding with label parameters (§4.9.4) may not be an entry point, also when no record is in its type."

**§11.1.** Add "records" to the list of values.

**§11.4.** Add: "Records are the same when they have the same labels and the values with the same label are the same. So `{ a: 1, b: 2 } == { b: 2, a: 1 }`, and `{ a: 1 } != { a: 1, b: 2 }`, also where both have the type `{ a: Int }`. A record is never the same as a value that is not a record."

**§11.8.** Add a row: "Record | `{ `, then for each field in the order of the bytes of its label: the label (as it is when it is a `name`, and in the String text form when not), `: `, and the text form of the value, with `, ` between fields, then ` }`. The record with no fields is `{..}`. So `{ age: 36, name: \"ada\" }`."

**§0 (the start of the spec).** In the list of what the document defines, add "the library module `i/record`, which reads and sets fields of records (§14)". The sentence "Every compiler must give the six library modules above" then counts `i/record` too; IO owns the count (below).

**§2.1.** "the six **library modules** (§9, §13)" becomes "the **library modules** (§9, §13, §14)", with the count left to IO.

**§12.** Add `i/record` (§14) to the list of modules. IO owns the sentences that count the modules (§0, §2.1, §12), so the number is set with IO; T3 adds the module and the section pointer in all three places.

**§14 The module `i/record` (new).** A table with the seven functions of part 10, their types, and one line each, in the form of §13.

## 13. Other documents

These are applied by T3, after G3. They are not part of this commit.

- **syntax.md.** In the summary, "`{ name: e, … }` is kept for records (stream T3)" becomes "`{ name: e, … }` is a record (`docs/design/records.md`)". In §6.1, the "kept for records" bullet becomes the reading rule of part 8.1, and in the `Function` line the body `Block` becomes `( Block | Record )`. In §1.7, add the record line.
- **syntax.md, "record update".** The rename of part 12 is made here too: in §6.1, `RecordUpdate` becomes `CtorUpdate` in the `Primary` line (:210) and in its own line (:220), and any text that calls `C(..e, …)` a "record update" calls it a "constructor update".
- **meta.md §11 question 2.** The answer of part 8.2: a one-part `{ }` in a type is a group, unless it is `{..}`, begins with `..`, or is `label: T`.
- **meta.md, "record update".** In §2.3, the row "Record update" (:171) becomes "Constructor update", and "when its second part is `..` it is a record update" (:174) becomes "… a constructor update". In the list of datums that are compile errors (:199), "a part of a record update with no label" becomes "a part of a constructor update with no label". Then spec, syntax.md and meta.md use "record" only for the values of §4.9.
- **meta.md §5 (names in expanded code, M2 and M4).** After "Labels of fields, of parameters, and of arguments are not names in this sense. They are compared by their text only.", add: "An `UpName` label in a record type is not such a label: it is a declared type variable (a label parameter, spec §4.9.4), so it is a name, looked up with its scope by rule 1. A template that writes `{ K: Int }` with its own `K` does not reach a `K` of the caller." (Part 5.10.)
- **meta.md §7 (typed macros, M4).** The requirements of part 5.10: the checks of L1, L2, L2a, L5 and L6 run on every expansion; an expansion adds no label parameter; `type_of`, `subtype` and `members` handle record types, spreads and `Param` labels, and `subtype` answers `NotKnown`, not `No`, when a label parameter is in either type. §7.4's sentence on `No` becomes: "`No` when it is not, and neither type has a type variable or a label parameter (§4.9.4) in it."
- **Lisp surface (proposed to M1 and D-meta; M1 decides).** In a `{ }` datum, a first part `(: label e)` or `..` makes a record: `{ (: name e) (: age e) }`, `{ .. r (: age e) }`, `{ .. }`, and in types `{ (: name T) }`, `{ .. R (: K V) (- age) }`, `{ .. R }`. The core-form tables of meta.md §2.3 would get these rows.
  - **Round trip (meta.md §1).** Every such record has a normal-surface form that is the same code, because of the `,` rule of part 8.1: `(= r { (: f (-> (x) Int x)) })` is printed `r = { f: (x) -> Int { x }, }`. Without that rule there would be none: a group around the field value is itself a clause block, `( … )` is a tuple, and moving fields or adding a spread changes the core tree. A test runs `i fmt --to=i` and back on records whose first field is a function with a `->` part, with one field and with more.
  - **meta.md §3.3.** The note "`` `{ ~..parts} `` … with one part it is a group" gets: "unless that part is a field `(: label e)` or `..`; then it is a record. With no part it is `{}`, the clause block; the empty record is `{ .. }`."
- **STR (string interpolation, `"""`, `#"…"#`).** STR lands after T3, and makes an interpolated string a `string_lit`. Labels in records and record types must stay strings with no interpolation (part 8.1); raw and multi-line strings with no interpolation are allowed. STR's grammar must keep a form of `string_lit` without interpolation for labels, or check this in the parser. This note is sent to STR with G3.

## 14. Files to change

Line numbers are at 1865366.

**`crates/ilang-syntax`**
- `ast.rs`: `ExprKind::Record`, `TypeKind::Record`, `RecordField`, `LabelName`.
- `parser.rs`: the record scan, used by `primary` (:766) and by `function` (:858) for the body `{`; `type_prim` (:1228), whose `T::LBrace` arm today always groups; a sequence reader in which a break is a `,`; the hints for `Person { … }` (`type_def`, :474) and for `{ name: n } = r`.
- `format.rs`, `print.rs`, parser tests, `tests/format.rs`.

**`crates/ilang-check`**
- `lib.rs` (:73-101): codes E0540 to E0545.
- `hir.rs`, `resolve.rs`: record nodes; labels are not names; `UpName` labels resolve to declared type variables (label parameters) or are an error; E0540, E0543.
- `types/mod.rs`: `Label`, `LabelParamId` (with the level of its binding, L8), `Head::Record`, `Ty::Rec`, `Ty::Without`, the smart constructors `rec` and `without`, `is_headed`, `value_head`, `Access::Label`; the scheme gets the list of its label parameters and the index of the parameter of each.
- `types/solve.rs`: the arms of 4.4; `inter_in`, `union_in`, `merge` (4.5); `add_bound`, `reach`, `compare`, `needs_join` (4.6); `map_children` (:642), `extrude` (:577), `freshen` (:618, with a label map), `bound_vars` (:1207), `level_of` (:203, counting the level of each `Param` label, L8); `FailKind::LabelMayBe`, `FailKind::LabelEscapes`; `value_head` used in `route` (:516), `_ <: Diff` (:316), `diff_in` (:877) and `diff_left` (:544).
- `types/display.rs`: the Coalescer (:503), `Norm` (:632, `union_in` :698, `inter` :772), `tree` (:1176, :1280), and the shown errors. `types/shown.rs`: `ShownType::Record`.
- `types/known.rs`: `members` (:21), `match_k` (:322), `disjoint`, record literals, and record arms in T1's `whole_lits` and `hidden_inputs` (part 4.8).
- `types/mod.rs`: `Typed.key_labels` and `KeyLabel` (part 9.3).
- `types/infer.rs`: `field()` (:1538) in three rules; record literals and spreads; label parameters in `call()` (:1260) and `instantiate` (:1000), with `key_labels` filled in `call()`; E0541, E0542; L9 in `entry_point` (:378); `has_no_effect` (:2340); `check_places` (:603) and `walk_variance` (:2269) get a `TypeKind::Record` arm (part 8.2); the known and expected types of spreads (parts 4.8 and 7).
- `types/pattern.rs`: narrowing (`:608`, `h => Ty::Con(h, args)`) builds the taken type `{..}` for `Head::Record`, not `Ty::Con(Head::Record, [])`, which would reach the `unreachable!` arm of `variances`; the type test (`:727-748`): the guard before the `filter_map` raises CASE_TYPE_VAR ("may not have a type variable as a member") for every member whose `head()` is `None`, and a spread member (`is_spread()`, part 4.2: a `Without`, or an `Inter` with a `Without` member, as `{ ..{ a: Int }, K: Int }` or `{ ..{ a: Int }, -K }` converts to) has the `head()` `None`. So the guard skips a member for which `is_spread()` is true, and the `filter_map` maps a `Ty::Rec` member and a spread member to `Pat::Head(Head::Record, 0)`. Then the test is not `Pat::Or([])`, is recorded in `type_tests`, and a spread type in a type test takes every record, as any record type does. A test checks it (`type_test_spread_key.i`, 15.1). `types/coverage.rs`: `values()` (`:209`) gets a `Rec` arm (and a `Without` arm) giving the head entry `(Head::Record, ValueSet::ALL, [], mode)`, in T1's four-part form. The set must be `ALL`: the union and intersection of `Vals` drop an entry whose set is empty, and records have no value sets. And `show_head` (`:615`) shows `Head::Record` as `{..}`. Each is after T1's changes there.
- `query.rs`: `Access::Label` has no definition target; hover shows the field type; completion after `.` lists the labels of the one `Rec` member of the known type.

**`crates/ilang-wasm`**
- `ir.rs`: `Atom::Label`, `Value::RecNew`, `Value::RecGet`, `Value::RecWith`, `Cond::IsRec`.
- `lower.rs`: `ExprKind::Field` with `Access::Label` (:1502), literals and spreads, label constants, key arguments from `key_labels`, `head_cond` (:2209), the `"ilang:record"` arm in `external_op` (:1364).
- `emit/mod.rs`: `$is_record` (next to :16), the label globals (all named by number, `$label_N`; constant, and mutable ones set in `$start` above `FIXED_MAX`), the `Atom::Label` arm of `atom()`, the new values, and large literals with `array.new` (9.3).
- `runtime/mod.rs`: tag 14 in the table (:37-52). `runtime/values.rs` and the new `runtime/values/record.rs`. `runtime/values/inspect.rs`: the branch of 9.5.

**Library, package, docs**
- `lib/i/record.i` (new); `crates/ilang-package/src/library.rs` (:12); the library tests.
- `docs/spec.md` (part 12), `docs/design/syntax.md` and `docs/design/meta.md` (part 13), `docs/performance.md` (9.7).

**Ownership and merge order (a change to the plan, for the coordinator).** Some of these files are owned by other tracks, or have a merge order that leaves T3 out. The plan's table gives T3 only the record syntax, `field()`, the `&` of fields in `solve.rs`/`display.rs`, the run time, `i/record` and some spec sections. This design also needs:

| File or section | Owner in the plan | Order in the plan | What T3 changes | Asked order |
|---|---|---|---|---|
| `resolve.rs` | A1, A3, T2 | A3 → T2 → M2 | label parameters, E0540, E0542, E0543, the E0225 message | A3 → T2 → T3 → M2 |
| `order.rs` | T2 | A3 → T2 → M2 | the L6 group check | A3 → T2 → T3 → M2 |
| `query.rs` | T2 (A3 at :971) | none | hover and completion for `Access::Label` | T2 → T3 |
| `types/pattern.rs`, `types/coverage.rs` | T1 | none (T1 owns `types/*`) | the arms of `Head::Record` | T1 → T3 |
| `infer.rs` `entry_point` | in the `infer.rs` order | T1 → T2 → T3 → R2 → M2 → M4 | L9 | no change |
| spec §9.2 (one existing sentence) | R and IO own new §9 text | none | "a record of functions" becomes "a custom type whose constructor holds functions" | after R and IO when they have landed; else they keep the T3 sentence |
| spec §10.3 | T3 (for records), also C, R2, IO, M2 | C → R2 → IO → M2 | two sentences | T3 after whichever of these have landed, and the later ones keep them |

T3 merges after T2 in every case, so the asked orders only add T3 where it already stands in time. Step 1 of part 16 checks these orders with the coordinator before any code.

## 15. Tests

### 15.1 Spec tests (`tests/spec/4.9/`, `tests/spec/6.13/`, `tests/spec/6/`, `tests/spec/11/`)

Each has a `// spec:` and an `// expect:` header, as in `tests/spec/4.8`.

Correct programs (`// expect: pass`):
- `literal_type.i`: `{ name: "ada", age: 36 }` has the type `{ age: 36, name: "ada" }` (T1's one-value types), and may be given where `{ age: Int, name: String }` is written.
- `width.i`: `(u) { u.name }` used on records of two shapes.
- `field_principal.i`: `(u) { u.name }` is `(u: { name: A }) -> A`.
- `inter_fields.i`: `(x) { x.a + x.b }` is `(x: { a: Int, b: Int }) -> Int`.
- `many_reads.i`: `(x) { (x.a, x.b, x.c, x.a) }` gives `{ a: A, b: B, c: C }`; a second bound on `a` is checked against the lower bound.
- `union_join.i`: two branches give `{ a: Int | String }`, and `.a` works.
- `spread_override.i`: `{ ..{ a: 1 }, a: "s" }.a` has the type `"s"`.
- `put_get.i`, `put_override_type.i`: `record.put({ age: 1 }, "age", "x")` is `{ age: "x" }`.
- `put_remove.i`: `record.remove({ a: 1, b: 2 }, "a")` is `{ b: 2 }`.
- `twist_call.i`: part 5.7's `twist({ a: 1 }, "b").a + 1`.
- `schema.i`: the program of part 5.8, exactly as written, with the scheme of `field` and the type of `user`.
- `codec.i`: the program of part 5.9, exactly as written; `user` is shown as `Codec(Never, { age: Int, name: String })`; `Codec(_, User) = user` binds `User` to `{ age: Int, name: String }`; a value is decoded and encoded again; and `user.encode({ name: "ada" })` is E0512 (the field `age` is missing), so the encode side checks its argument although it is shown as `Never`. The same program with `user: Codec({ age: Int, name: String }, { age: Int, name: String })` written is correct and is shown with that type.
- `label_param_pass.i`: a label parameter passed through two levels of functions, by name and through `k2 = key`; and the same key passed as `k` in `key |> { k -> record.get(r, k) }` and as `k2` in the alias parameter `(r: { K: Int }, k2 = key: K)` (L4: forms the syntax makes the same binding get the same verdict).
- `type_test_spread_key.i`: `f(K) = (r: {..}, key: K) -> Int { r |> { y: { ..{ a: Int }, K: Int } -> 0  _ -> 1 } }`, and the same with `y: { ..{ a: Int }, -K }`; each type test takes every record (part 14, `pattern.rs`), and no CASE_TYPE_VAR is given.
- `record_equality.i` (in `tests/spec/11/`, §11.4): `{ a: 1, b: 2 } == { b: 2, a: 1 }`; `{ a: 1 } != { a: 1, b: 2 }`, also when both reach `==` through a parameter of type `{ a: Int }`; `{ a: 1 } != { b: 1 }`.
- `record_text_form.i` (in `tests/spec/11/`, §11.8): `core.inspect` of `{..}`, of `{ age: 36, name: "ada" }`, of a record with the quoted labels `"Type"` and `"first-name"` (`{ "Type": 1 }`, not `{ Type: 1 }`), and of a nested record.
- `evaluation_order_record.i` (in `tests/spec/6/`, §6.2, as `evaluation_order.i`): effects in a literal `{ b: f(1), a: f(2) }` happen in the order written, not in label order; in `{ ..g(), a: f(1) }`, `g()` happens first.
- `string_labels.i`: `record.get({ "first-name": "ada" }, "first-name")`.
- `type_test_all_records.i`: `x: {..}` takes the records out of `Int | { a: Int }`.
- `field_declared_var.i`: `id_name(A) = (x: A) { x.name }` and `names(A) = (xs: List(A)) { list.map(xs, (x) { x.name }) }` have the same types as without the annotation (rule 13b with a type variable in the known type).
- `field_known_type.i`: `r = { user: User("bo"), cb: (x: Int) { x } }`; `r.user.name` uses rule 13a, and `r.cb(x: 1)` calls by label.
- `spread_alone_type.i`: `(r: { ..R }) { { ..r } }`, and `put`'s own type `(r: R & {..}, key: K, value: V) -> { ..R, K: V }` with the spread in an output place (part 8.2).
- `spread_known_type.i`: `User { User(name: String) }`, `a = { u: User("bo") }`, `b = { ..a, n: 1 }`, then `b.u.name` (rule 13a through the known type of a spread); and with `Cfg = { cb: (User) -> String, n: Int }`, `base: Cfg = { cb: (u) { u.name }, n: 0 }` and `next: Cfg = { ..base, cb: (u) { u.name } }` (the expected type of a spread part).
- `key_clause_param.i`: `f(K) = (key: K) { key }` and `f(K) = { key: K -> key }` get the same verdict and the same type (L2 treats the one-clause clause block as the function).
- `field_never.i`: `list.map([], (x) { x.name })` is correct with the type `List(Never)` and keeps `Access::Field` (rule 13a with the known type `Never`).
- `one_field_fn.i`: `{ f: (x) -> Int { x }, }.f(1)` is a record with one field.
- `show_reads_back.i`: a value of type `{ ..R, -a } & { b: Int }` is shown as `{ ..R, -a } & { b: Int }` in an error message, and that text, written as a type, gives the same type.
- `nominal_field_unchanged.i`: rule 13a with known types and with §7.5 narrowing.
- `narrow_record.i`: `f = (x: Int | { a: Int }) { x |> { y: { a: Int } -> y.a  n -> n + 1 } }`; `n` is `Int` (rule 12 takes `{..}`), and at run time `f({ a: 2 })` is 2 and `f(3)` is 4.
- `key_known_fn.i`: `Box(A) { Box(item: A) }`, `apply(K) = (b: Box({ K: Int }), key: K, f: (Box({ K: Int })) -> Int) -> Int { f(b) }`, and `n = apply(Box(item: { a: 1 }), "a", (x) { x.item.a })`, which is correct (L4 step 3: the known type of `apply` gets the label map), and the scheme of `n`'s binding holds no `K`.
- `key_inner_label.i`: `f(K) = (r: {..}, key: K) -> Int { h = (s: { K: Int }) { s } ⏎ 0 }`; `K` is a label parameter (L1 counts the inner binding).
- `union_spread.i`: `f(R) = (c: Bool, r: R & {..}) { c |> { True -> record.put(r, "a", 1)  False -> { b: 2 } } }`; the scheme is shown `(c: Bool, r: R & {..}) -> { ..R, a: 1 } | { b: 2 }`, with two members, and that text, written as a type, gives the same type. (Put's own `R2` is only in an output place, so the shown type replaces it by its lower bound `R & {..}`, and E5 makes `{ ..R & {..}, a: 1 }` into `{ ..R, a: 1 }`.)
- `union_spread_closed.i`: the same body with `r: {..}` and no `R`; the scheme is shown `(c: Bool, r: {..}) -> {..}`. In the shown type, `R2` is replaced by `{..}`, E4 makes the spread `{ a: 1 }`, and the union of `{ a: 1 }` and `{ b: 2 }` is put together into their common labels, none.
- `accumulate_put_annotated.i`: `build = (r: {..}, n: Int) -> {..} { n |> { 0 -> r  _ -> build(record.put(r, "a", n), n - 1) } }`.
- `fold_spread_annotated.i`: `list.fold(xs, { n: 0, total: 0 }, (acc: { n: Int, total: Int }, x) { { ..acc, n: acc.n + 1 } })`, the same with `record.put(acc, "n", acc.n + 1)`, and the literal form `(acc, x) { { n: acc.n + 1, total: acc.total } }` with no written type (part 4.2).
- `function_body_record.i`: `(n) { name: n }`.
- `spread_upper_bound.i`: a spread reaches an upper bound through a variable: a parameter `p: { ..R, a: Int }` in an input place, and a variable bounded above by a `Without`.
- `union_spread_written.i`: the `f(R)` of part 4.4, arm 5a, with the result type `{ ..R, a: Int } | { b: Int }` written; the scheme of `f` has no bound `{ b: Int }` on `R`, and `f(True, { x: 1 })` is correct and has the field `x`.
- `known_fold_record.i`: `list.fold(xs, { n: 0 }, (acc, x) { { acc.n == 3 } |> { True -> acc  False -> { n: acc.n + 1 } } })` gives no E0520 (part 4.8, `whole_lits`), the same as the tuple form `(0, 0)` with `acc.0`.
- `label_param_modules/` (a project with two modules, as `tests/spec/2.5/modules/`): module `schema` exports `object` and `field` of part 5.8, and module `main` has `user = schema.object |> schema.field("name", string) |> schema.field("age", int)` with the type `Schema({ age: Int, name: String })`, and a binding of `main` with its own label parameter that calls `schema.field` with its key by name. This checks `other_known` with the label map (L4 step 3) and the label parameters of a scheme across a module boundary.
- `texts.i`: `record.texts({ user: "ada", tries: 3 })` is `[("tries", "3"), ("user", "\"ada\"")]`, and a log line built from it in plain i.

Programs that must be rejected (the refuter's list), each with its code:
- `bump_unsound.i` (E0501): part 5.6's `bump` with `put`.
- `twist_unsound.i` (E0501): `twist({ a: 1 }, "a").a + 1`.
- `twist_body.i` (E0544): part 5.7's `peek`, with `r: { a: Int }`, and again with `r: {..}`; both errors are in the body, at the `.a`.
- `key_not_label.i` (E0541, with the hint "`K` is not a label parameter"): `twist` of part 5.7 without its `->` part.
- `key_escape.i` (E0545): `outer = (acc) { f(K) = (r: { K: Int }, key: K) { acc(r) } ⏎ f({ a: 1 }, "a") }`; the error is at `acc(r)`, inside `f`.
- `type_test_without.i` (E0512 for `f`, E0501 at the call `g({ b: "s" })`): `f = (r: {..}) { x = record.put(r, "a", "s") ⏎ x |> { y: { a: Int } -> y.a + 1  _ -> 0 } }`; and `g = (x) { record.remove(x, "a") |> { y: { b: Int } -> y.b + 1  _ -> 0 } }` with `g({ b: "s" })`. A `Without` must not pass a type test through `r - T` (part 4.4, arm 6, step a).
- `diff_without.i` (E0501, and the checker must end): `drop(B) = (x: B - {..}) { 0 }` and `g = (r) { drop(record.put(r, "k", 1)) }`; and `strip(A) = (x: A, r: {..}) -> A - {..} { record.put(r, "a", 1) }` (step c).
- `two_keys.i` (E0544): part 5.3's `both`, with the error at the body of `both`.
- `key_inner_lambda.i` (E0543): `h = (y: K) { record.get(r, y) }` inside a binding with the label parameter `K`.
- `key_fn_type.i` (E0543): `g(K) = (r: { K: Int }, k: K, f: (y: K) -> Int) -> Int { f("zzz") }`.
- `key_pattern.i` (E0543): in a binding that uses `K` as a label (`(r: { K: Int }, key: K)`), `K` in the pattern of a clause block with two clauses, in a binding statement `y: K = key`, and as a result type.
- `key_through_call.i` (E0541): `bad(K) = (r: { K: Int }, key: K, s) -> Int { record.get(r, second(key, s)) + 1 }`, with `second(A) = (x: A, y: A) -> A { y }`; also through `unwrap_or(Ok(s), key)`.
- `mutual_key.i` (E0543): `f(K)` and `g(J)` with label parameters that call each other; also `f(K)` in a group with a `g` that has none.
- `put_then_read_old.i` (E0501): `record.put({ a: 1 }, "a", "s").a + 1`.
- `union_key.i` (E0541): a key with the known type `"a" | "b"`, made by a clause block.
- `string_key.i` (E0541): a key of type `String` from a parameter.
- `unknown_key.i` (E0541): a key with no known type.
- `recursive_key.i` (E0541): a function with a label parameter that calls itself with another key.
- `label_fn_value.i` (E0542): `list.map(xs, record.get)`.
- `label_fn_value_type.i` (E0542): `apply = (f: { record.get }, r) { f(r, "a") }`, a value type that names a binding with label parameters (L5); and the same name in an alias pattern.
- `key_value_type.i` (E0543): `g(K) = (r: { K: Int }, k: K, f: (y: { k }) -> Int) -> Int { f("zzz") }`, a value type that names the key parameter `k` (L2a); and the same through `k2 = k`.
- `spread_union_input.i` (E0518): `h(R) = (p: { ..R, a: Int } | { b: Int }) -> Int { 0 }`, and `h2(R, S) = (p: { ..R, a: Int } | { ..S, b: Int }) -> Int { 0 }` (a union with a spread member only in an output place, part 4.5).
- `label_param_modules_value/` (E0542, a project with two modules): `list.map(keys, schema.field)` in `main`, with `field` from the module `schema` of `label_param_modules/`.
- `label_param_export/` (E0320, a project with `[wasm] exports`, as `tests/spec/10.2/export_never_param/`): the `pick` of L9, whose host signature has no record.
- `matched_key.i` (E0541): `f = (r: { a: Int }, s: String) { s |> { "a" -> record.get(r, s)  _ -> 0 } }`; a matched String is not a one-value key (part 11).
- `self_value.i` (E0542): the program of L5, where `f` is bound to `g` inside its own value; and the same with `apply(f, r, "zzz", 0)`.
- `recursive_record.i` (E0505): `last = (r) { r.next |> { Nil -> r.v  n -> last(n) } }` (part 4.2).
- `accumulate_put.i` (E0505, with the hint to write the parameter type): `build = (r, n) { n |> { 0 -> r  _ -> build(record.put(r, "a", n), n - 1) } }`.
- `fold_spread.i` (E0505, with the same hint): `list.fold(xs, { n: 0, total: 0 }, (acc, x) { { ..acc, n: acc.n + 1 } })`, and the same with `record.put(acc, "n", acc.n + 1)` (part 4.2).
- `fill_generic.i` (E0505): `fill(R, K) = (r: R & {..}, key: K, n: Int) -> { ..R, K: Int } { n |> { 0 -> record.put(r, key, 0)  _ -> fill(record.put(r, key, n), key, n - 1) } }` (part 4.2: a loop cannot keep a generic rest).
- `key_two_params.i` (E0541): `pick(K) = (k1: K, k2: K) -> Int { r: { K: Int } = record.put({..}, k1, 1) ⏎ record.get(r, k2) }`, called from i code as `pick("a", "b")`; and a binding `g(J) = (r: { J: Int }, j: J) -> Int { pick(j, "a") }`, where one key gives `Param(J)` and the other `Name("a")` for the same `K` (L4: all key parameters of one label parameter must give the same label).
- `record_service.i` (E0225): `Clock = { now: () -> Int }`, then `fixed = () { ! Clock = { now: () { 42 } } ⏎ elapsed(40) }`; a record type is never a service type (§9.2 as changed in part 12).
- `spread_alone_known.i` (E0520): `g = (x) { { ..x } == 1 }`, because the known type of `{ ..x }` is `{..}` (part 4.8), which is disjoint from `Int` (§6.9).
- `upname_first_label.i` (the error "a label is a name or a string"): `{ Type: 1, b: 2 }` and `{ b: 2, Type: 1 }` give the same message.
- `label_param_place.i` (E0543): `-> K`; a label parameter that no parameter has as its whole type.
- `missing_field.i` (E0512, as for `User("bo").age` with `User { User(name: String) }`): `{ a: 1 }.b`.
- `unused_record.i` (warning "its value is not used"): `f = () { { a: 1 } ⏎ 2 }`; no warning for `{ ..r, a: 1 }` as such a statement (§6.10 as changed in part 12).
- `record_not_never.i` (E0501): `{ a: 1 }` where an `Int` is needed; also `{ a: 1 } & { b: 1 }` built by inference and given where an `Int` is needed. This guards against a record becoming `Never`.
- `nominal_not_record.i` (E0501): a `User` given to `(u) { u.name }`, with the hint.
- `mixed_target.i` (E0512): `.name` on `User | { name: String }`.
- `union_lost_field.i` (E0512): `.b` on a match whose branches give `{ a: 1, b: 2 }` and `{ a: "x" }`.
- `type_test_record_shape.i` (E0501): `x: { a: Int }` on `{ a: Int } | { a: String }`.
- `interpolated_label.i` (parse error, after STR): `{ "a\(n)": 1 }`, and the same label in a type.
- `duplicate_label.i` (E0540): `{ a: 1, a: 2 }`, `{ a: 1, "a": 2 }`, `{ a: 1, "\u{61}": 2 }`, `{ ..r, a: 1, a: 2 }`, `{ a: Int, a: String }`, `{ a: Int, "a": String }` and `{ ..R, a: Int, -a }`.
- `spread_output_place.i` (E0518): `(r: R & {..}) -> { ..R } { r }`, which gets the same verdict as `-> R & {..}`; and `bad(A) = (x: A) -> { a: A & Int } { … }` (a record field is an output place).
- `record_to_host.i` (E0318, E0320).
- `empty_braces.i` (E0501): `{}` where a record is needed, with the hint.

### 15.2 Checker unit tests

- Each rewrite E1 to E5 and N1; that a `Rec` with a field that may be `K` is not rewritten.
- R-split, R-without-left, R-without-right, R-width; the left-`Inter` arm with a `Rec` member.
- R-spread-union (arm 5a): `Without(R2, {a}) & { a: 1 } <: { ..R, a: Int } | { b: Int }` gives `R2 <: R` and adds no bound `R2 <: { b: Int }`; with no candidate it goes on to the arms below; with two candidates it fails with `Ambiguous`, whatever the order of the members. `route`: a `Rec` against a union with a `Rec` member and a spread member goes to the `Rec` member; against a union with one spread member and no `Rec` member, to that member; against two spread members, `Ambiguous`, in either order.
- R-without-left fails at once with `LabelMayBe` or `NoField` when the base is a variable with no bounds, and with one; the left-`Inter` arm keeps that failure when the next member is a `Without`.
- R-without-left, step by step: `Without(B, L) <: R | a - H` goes to `R`, not to the difference; `Without(B, L) <: a - {..}` and `<: a - { x: Int }` fail with `Mismatch`; `Without(B, L) <: a - Int` gives `Without(B, L) <: a`; `Without(B, L) <: Int` fails; `Without(B, L) <: (a - H) | Int` ends (a check with a depth counter). `value_head` of a `Without` is `Record` in `route`, `_ <: Diff`, `diff_in` and `known::disjoint`.
- L8: a `Param` label that reaches a variable of a lower level fails in `extrude` with `LabelEscapes`; one that stays at its own level does not. The level kept in a `LabelParamId` is the raised level of its binding, and a variable made at the outer level (where the name is bound) is strictly below it, so `key_escape.i` gives E0545 and not a scheme with `K`.
- The Member arm: `F & { K: V } <: F` adds no bound to `F`.
- E0, the new E1 (`without(Int, L)` is `Never`) and E5 (`without(R & {..}, {a})` is `without(R, {a})`); E1 on `{ a: Never, b: Int }` with `{a}` gives `Never`, as on `{ a: Never, b: String }` and on `Never` (types with the same values give the same result); E4 on `{ a: A, b: Int }` with a variable `A` gives `{ b: Int }`.
- **Read back:** for each type made in these tests, show it, parse the text as a type, and check that the two types are equal (part 4.7).
- `&` and `|` of records, with `Param` labels.
- **Stable under replacement:** check a body with `Param` labels, record each constraint the solver checked, with its two types as they were built; replace every `Param` in those types by each label of the body and by a new one (with the smart constructors, and without joining a union of records again), check each constraint again, and compare: every check that passed must pass. The test works on the built types, not on the source: a written union of records is joined once, when it is built (5.5), so `h = (s: { K: Int } | { a: Int }) { 0 }` keeps `s: {..}`, and `h({ b: 1 })` passes in both runs.
- **No over-approximation:** a debug assertion in `Norm` and the Coalescer that a `Without` is rewritten only by `without()`; a test that runs `display::simplify` on schemes with `Without` in input and output places, and checks that no call of the reject list of 15.1 becomes correct.
- `add_bound` with many `.f` on one variable: two fields, then a lower bound with only one; then two bounds on one label and a lower bound that fits only one of them. Both must fail.

### 15.3 Parser tests

`{ x: Int -> x }` is a clause block. `{ x: Int = 1 ⏎ x }` is a block. `{ x: 1, y: 2 }`, `{ ..r, x: 1 }`, `{..}` and a record over several lines are records. `{ "a b": 1 }` is a record. `{ n: 0, f: (x) -> Int { x } }`, `{ f: (x) -> Int { x }, n: 0 }` and `{ f: (x) -> Int { x }, }` are records; `{ f: (x) -> Int { x } }` is a clause block. The printer writes `{ f: (x) -> Int { x }, }` for a record with that one field, also on several lines. `{ ..R }` is a record type. `(n) { name: n }` is a function with a record body. `(x) { x: Int = 1 ⏎ x }` has a block body. `(x) { x: Int -> x }` has a block body and is an error. `{}` is a clause block. In types: `{ x: T }` is a record type, `{ T }` and `{ x }` are groups, `{..}` is all records, `{ ..R, K: V, -a }` is a record type, and a break inside a record type is not `TypeGroupBreak`. `Person { name: String }` gives its hint. The formatter round-trips each of them.

### 15.4 Codegen tests (`crates/ilang-check/tests/codegen.rs`, `crates/ilang-wasm/tests/codegen.rs`)

`Access::Label` for `r.name` on a record; label globals are constant expressions (part 9.3) and the module validates; `{..} |> record.put("name", "ada")` and `k = "name"` then `{..} |> record.put(k, "ada")` store the label global of `name`, the same global that `.name` passes (from `Typed.key_labels`); `Access::Field` still for `p.name` on a custom type and for `x.name` with the known type `Never`; `Access::Part` for `t.1`. A module that calls each of the seven `ilang:record` functions validates (each returns `eqref`, part 9.3). A literal is one `$record_new` with sorted labels. Records with the labels `"a b"`, `"é"`, `"x(y)"` and `"q\"q"` give a module that assembles and validates (label globals are named by number). The WAT of every program with no records changes only in the runtime part (checked with `scripts/wat-diff`).

### 15.5 Runtime tests (`crates/ilang-wasm/tests/runtime_values.py` and `.rs`, at `-O0` and `-O2`)

- `{ a: 1, b: 2 } == { b: 2, a: 1 }`.
- `{ a: 1 } != { a: 1, b: 2 }`, also through a parameter of type `{ a: Int }`.
- `{ a: 1 } != { b: 1 }`, and their hashes differ (a fixed test).
- A record is never equal to a tuple with the same array.
- `hash(x) == hash(y)` whenever `x == y`, for random records built in random orders by literals, `put` and spreads.
- A literal and the same record built by `put` are equal (the compile-time sort and `$string_compare` agree).
- `inspect`: nested records, `{..}`, quoted labels, a record that holds a function.
- `put` that replaces and that adds; `remove`; `has`; `keys`; `size`.
- `$record_get` on a record that has more fields than its static type.
- A label of 10001 bytes and a literal with 5001 fields (the `FIXED_MAX` fallbacks of 9.3): the module validates, and `==`, `hash`, `inspect` and `.f` agree with the same records built with `put`.
- `texts` on nested records, quoted labels and the empty record.

## 16. Checklist for track T3

In order, after A1 and T1 have landed, T2 has merged, and S3 is approved.

1. **Spec text (gate G3).** Part 12 goes to the user, written on top of the spec after T1 and T2. Check that §4.9, §6.13 and §14 are still free, with IO and M2 (part 19). Get the coordinator to add T3 to the merge orders of `resolve.rs`, `order.rs`, `query.rs`, `pattern.rs`, `coverage.rs`, spec §9.2 and §10.3 (the table at the end of part 14).
2. **Runtime order.** Check that `$string_compare` orders exactly as Rust `str` does, with a test. (At 1865366 it does: bytes, then length.)
3. **Soundness tests first.** Write the reject list of 15.1 and the unit tests of 15.2 as failing tests.
4. **Types core.** `Label` (with `Arc<str>`, so that `Ty` stays `Send` for the LSP), `Head::Record`, `Ty::Rec`, `Ty::Without`, `rec()`, `without()`, `is_headed`, `Access::Label`, with unit tests. Then the audit. The exact pattern `Ty::Con(..) | Ty::Fn(_)` finds only 8 lines at 1865366 (solve 6, display 1, known 1), so it is not the gate. The gate is a list: at the start of this step, save the output of `rg -n -e 'Ty::Con\(' -e 'Ty::Fn\(' -e '\.head\(\)' -e 'Head::' crates/ilang-check/src` (200 lines at 1865366: solve 36, display 49, known 34, infer 41, pattern 15, coverage 19, mod 6; more after T1) into the T3 pull request as a checklist, and mark each line "handles `Rec`/`Without`", "cannot see a record" (with the reason), or "changed". The step is done when every line is marked. These come first: `reach` and the `new` list of `add_bound` (`solve.rs:397`, :438); narrowing and the type test in `pattern.rs` (`:608`, `:742`); `values()` and `show_head` in `coverage.rs` (`:209`, `:615`). A unit test runs the narrowing of rule 12 on `x: Int | { a: Int }` with a clause `y: { a: Int }` and a catch-all `n`, and checks that `n` is `Int` and that the type test is recorded.
5. **Solver.** The Member arm and the arms of 4.4 (R-without-left with steps a to f, never sending a `Without` past a record member of a union or back to itself; it fails at once, with `FailKind::NoField` and `FailKind::LabelMayBe`; the left-`Inter` arm keeps the record failure; R-spread-union, arm 5a, chooses a spread member of a union by its labels), `&` and `|` of 4.5, the bounds of 4.6. `value_head` in `route`, `_ <: Diff`, `diff_in`, `diff_left` and `known::disjoint`. Run the unit tests.
6. **Display and known types.** `display.rs` (with the read-back rule of 4.7), `shown.rs`, `known.rs` (with the record arms of T1's `whole_lits` and `hidden_inputs`, part 4.8). No `.out` file should change at this step, because no program has records yet; if one changes, find out why before going on.
7. **Syntax.** AST, the scan (with the `,` rule), types (with `{ ..R }`), the printer and the formatter (the `,` after a first field with `->`), parser tests. First check with a test that no correct program has a `,` directly inside the braces of a block or a clause block. Remove the "kept for records" error.
8. **HIR and resolve.** Labels, label parameters (L1, counting every type written in the binding), key parameters, E0540, E0543 (the places of L2, a key parameter named in a value type or an alias pattern by L2a, and the recursive groups of L6 in `order.rs`), and E0542 (L5: every use of such a name that is not a callee, also inside its own value and in a value type or an alias pattern). Write these checks as one function over the HIR of a binding, so that M4 can run it on expanded code (5.10). The E0225 message of `env_type` (part 12, §9.2).
9. **Inference.** Literals and spreads (with their known and expected types), `field()` in three rules, `has_no_effect`, `check_places` and `walk_variance` (part 8.2), then label parameters in `call()`: the name rule and the one-string rule of L4, the known function type built with the label map (L4 step 3), `Typed.key_labels` filled in `call()` (9.3), E0541, L6, L9 in `entry_point` (E0320), and L8 in `level_of` and `extrude` (E0545), with the raised level kept in each `LabelParamId`. Then the occurs check with `Rec` fields and `Without` bases inside a type constructor (part 4.2), and E0512 for `FailKind::NoField`. Then accept the changed outputs with `UPDATE=1` and read each change. They come from rule 13b: the E0506 "`.name` needs a known type" expectations in `crates/ilang-check/tests/types/` (`errors`, `expected_types/expected`, `fields_union`, `known_general_vars`, `known_partial_schemes`, `known_pipe_left`, `known_types`, `labels`) become types, or E0501 or E0512 at another place. For each one, check that the new verdict is the one part 6 gives; for example `through_id = users |> map(id) |> map((v) { v.age })` in `known_general_vars.i`.
10. **Run time.** Tag 14, `record.rs`, the inspect branch, IR, lowering, emit, `external_op`, the label globals in constant form, with the `FIXED_MAX` fallbacks for long labels and large literals, and the key arguments lowered to them from `key_labels` (part 9.3). Gate: `scripts/wat-diff` shows no change in the generated code of programs with no records.
11. **Library.** `lib/i/record.i`, its registration, and the library tests (names, labels).
12. **Tests.** 15.1 to 15.5 at `-O0` and `-O2`, the spec test suite, and the full dev-verify.
13. **Docs.** `docs/performance.md` (9.7, with the micro-benchmark), syntax.md and meta.md (part 13), and `query.rs` hover and completion.

## 17. Risks

- **The audit is large.** About 200 lines of the checker match `Ty::Con(`, `Ty::Fn(`, `.head()` or `Head::` (part 16, step 4), and many of them treat these as "has a type constructor". A missed one makes `Rec` fall into a catch-all: usually a wrong mismatch, but in `reach` or `add_bound` a bound could be dropped with no error. The `is_headed` helper, the marked list of step 4, and the reject tests guard this.
- **Unions of records lose fields.** `{ kind: "circle", r: Float } | { kind: "square", side: Float }` becomes `{ kind: "circle" | "square" }`. Users who know TypeScript may expect a tagged union. Custom types do that in i; the spec says so. Exact unions of records would break "one member per head" everywhere.
- **Label parameters are fixed before solving.** A computed key is rejected, a key passed on must be passed by its name, a label-generic function cannot be passed as a value, and it cannot be in a recursive group with other functions. This is the price of keeping label variables out of the solver and of tying each key to its label.
- **R-without-left assumes a record.** It fails at once when a field that a `put` or spread forgot is needed. If the base has no records, the type is empty and the check could pass; it is rejected anyway. Such bases do not come from `put`, `remove` or spread expressions.
- **One-parameter codecs.** A codec with `encode: (A) -> Json` and one type parameter cannot be built field by field; it needs two type parameters (part 5.9), or a later "lacks" form.
- **Forgetting with a rigid key loses fields.** In generic code, `{ ..R, K: V }` keeps no field of `R` that may be `K`. That is sound, and E0544 says why, but it may surprise users.
- **R-without-right is not complete.** A spread written in an input place is checked with `X <: U`, which may reject a correct call. It never accepts a wrong one.
- **Flexible declared variables.** The schema example relies on `R` getting the bound `{..}` by inference. If D-types makes declared variables rigid, such code must write `Schema(R & {..})`.
- **The misreading.** A record whose first field holds a function with a `->` part, with no `,` after that field, is read as a clause block. The error says to add the `,`, and the printer always writes it.
- **Changed outputs.** `(u) { u.name }` without a type now has a type instead of E0506, so expected outputs and messages change, and a custom value given to it fails at the call.
- **Run-time cost.** `.f` on a record is a binary search, and `put` copies. Fine for small records; shapes and caches are later work.
- **No recursive records.** A record type is a type constructor for the occurs check (part 4.2). So a function that walks a chain of records is E0505, and an accumulator loop over `put` needs a written parameter type. Recursive data is a custom type; §4.9.3 says so.
- **Spreads on a loop accumulator.** The most common record update, `{ ..acc, n: acc.n + 1 }` in a fold callback with no written type, is E0505, while the full literal `{ n: acc.n + 1, total: acc.total }` is correct (part 4.2). The user writes the accumulator's type or the literal; the hint of E0505 says so. A generic loop that sets a field and keeps the rest as `R` cannot be written at all. A later track may accept these cycles, whose least solution is finite (part 4.2); that would only accept more programs.
- **Codec encode types are shown as `Never`.** A codec built at the top level with calls (part 5.9) is not generalized, and its encode side has only upper bounds, so it is shown as `Never` and an alias pattern reads `Never` there. The encode side still checks its arguments. The type of the records is read from the decode side, or written.
- **Spread members of unions.** A union may hold a spread type next to a record type (part 4.5). It is the one exception to "one member per type constructor", and the spec says so. A path that assumes one member per head and does not look at `value_head()` would treat a record as one of two members; the audit list of step 4 covers this.
- **inspect.** The new branch shares kind 2 with tuples and custom values. A mistake there changes the text form of other values; the existing inspect tests must not change.
- **A key must be written as a label.** A function that passes its key on must use `K` as a label in one of its written types, most often the `->` part (L1). Without that, `K` is an ordinary type variable and the key argument is E0541. The hint says what to write. The rule keeps `id(A) = (x: A) { x }` an ordinary function, and lets resolve find label parameters from the syntax alone.
- **Keys stay inside.** A record type with a key `K` may not reach a name from outside the function that declares `K` (L8, E0545). A function that must hand such a value out passes the field's value, or takes the outer function as a parameter of its own.
- **Typed macros.** A typed macro could give back code that breaks L2 or adds a label parameter after inference. Part 5.10 asks M4 to run the label checks again on every expansion; T3 writes them so that they can be run again.
- **Order of work.** T3 depends on T1 (literal types and their known types), A1 (the parser) and T2 (`{ x }` in types). Its spec text must be put on top of theirs. If T1 represents literal types differently, L4 must follow it.

## 18. Rejected alternatives

- **One head for each label** (`Head::Field(label)`): breaks "different heads are disjoint" in `inter_in`, `Norm::inter`, `-`, `route` and `disjoint`. The left `&` would build `Never`, and `Never` fits everywhere.
- **`put` as `R & { K: V }`:** unsound (5.6).
- **Over-approximating spread rules** (a spread with a variable base only in output places, and rewrites that make a type larger): their soundness would rest on the polarity used by the Coalescer, which also makes the schemes of generalized bindings. `Without` with exact rewrites removes that dependence.
- **A `\` operator** (`R \ k`): a new token and a second form for one idea. `{ ..R, -k }` says the same inside the spread form.
- **A separate key type** (`Ty::Key`): one more case in every audit and in T1's String. The type `String` and the name rule of L4 are enough.
- **Label variables in the solver:** heads would depend on solving, and principal types would be lost.
- **Custom types as subtypes of their record view:** D7.
- **One struct for each record shape:** shapes are made at run time by `put`, and `.f` under width would need a test for each shape.
- **Widening the inspect packing to `<< 3`:** not needed, because kind 2 can look at the parent's tag.
- **`{}` as the empty record or the type of all records:** `{}` is the empty clause block, and the text form would not read back.
- **`{ f: { (x) -> Int { x } } }` as the way out of the misreading:** the inner `{ }` is itself a clause block by syntax.md §6.1 (its first thing is `(x)` followed by `->`), so it does not help. A `,` after the first field does.
- **A label parameter as a rigid abstract subtype of `String` in the body** (a skolem that only key parameters have): it would also make the body unable to supply a value for `K`, but it is a new type form in every audit and in T1's String. The limits of L2 give the same safety with no new form.
- **A label taken from the known type `K` of any expression:** known types are not checked by the solver, so `second(key, s)` could carry `K` to a value that is not the key (part 5.5). The name rule of L4 is used instead.
- **R-without-left that replaces the record by `Never` and goes on with `B <: Never`:** the failure is lost in the bounds of a variable (part 4.4, arm 6).
- **Record patterns in T3:** they touch T1's `pattern.rs` and `coverage.rs`. Type tests and `.f` cover the need for now.

## 19. Open questions

1. **Numbering.** Are §4.9, §6.13 and §14 still free after T1, T2, IO and M2? If §14 is taken, `i/record` takes the next free number.
2. **D-types.** Do declared type variables stay flexible (part 5.8)? If not, is `Schema(R & {..})` the form library authors should write?
3. **Union keys for `get`.** Allow `record.get(r, k)` with `k: "a" | "b"` as `r <: { a: V } & { b: V }` in a later track?
4. **Record patterns.** `{ name: n }` as a pattern, after T1's pattern work: a later track, or never?
5. **Lisp surface.** Does M1 take the rows of part 13?
6. **Shapes.** Should `-O2` add shape ids and inline caches for `.f` later, keeping `==` and `hash` canonical?
7. **Codecs (for D-types and json-schema).** Is the two-parameter `Codec(E, D)` of part 5.9 good enough for the json-schema track, which must then read the record type from the decode side (`Codec(_, User) = user`), because the encode side of a codec built with calls is shown as `Never`; or should a later track add a "lacks" form (a record type with no field `K`), so that a one-parameter `Schema(A)` with `encode: (A) -> Json` can be built field by field?
8. **STR.** Does STR keep a form of string literal with no interpolation in its grammar for labels, or does the parser check it (part 13)?
9. **Merge orders (coordinator).** Can T3 be added to the orders of `resolve.rs`, `order.rs`, `query.rs`, `pattern.rs`, `coverage.rs`, spec §9.2 and §10.3, as the table at the end of part 14 asks?
10. **S6 and D-types: value types and alias patterns.** This design makes a value type (`{ x }` in a type) or an alias pattern that names a key parameter E0543 (L2a), and one that names a binding with label parameters E0542 (L5), whatever S6 decides a value type means (a type pattern or a known type). Do S6 and D-types agree, and does T2 give resolve what it needs to find such a name in a type?
11. **M4.** Does M4 take the requirements of part 5.10 (the label checks run again on every expansion, and `subtype` answers `NotKnown` when a label parameter is in a type)?
