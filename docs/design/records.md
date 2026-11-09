# Design: structural records

Status: draft for review (study S3 of the redesign plan). When it is accepted at gate G3, track T3 puts the spec text of part 12 into `docs/spec.md` and makes the changes of parts 14 to 16. T3 starts after A1 (the new parser) and T1 (literal types) have landed, and after T2 has merged. Code facts are from commit 1865366. Line numbers are at that commit; A1, T1 and T2 move many of them.

The examples use the new syntax of `docs/design/syntax.md`.

## 1. The problem

i needs records with fields that are not declared in advance:

```i
ada = { name: "ada", age: 36 }          // { age: Int, name: String }
name_of = (r) { r.name }                 // (r: { name: A }) -> A
name_of(ada)                             // "ada"
```

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

**D2. Width subtyping is one new rule, `Rec <: Rec`.** A union of two records is put together into their common labels (the MLstruct join), the same way §4.4 puts `List(Int) | List(String)` together.
*Reason:* it keeps "at most one member of each head in a union", which `route` and known types rely on.

**D3. "Forgetting" is one exact type form, `Ty::Without(T, L)`.** It means "T with the labels in L forgotten". The written form is a spread: `{ ..R, age: V }` is `Without(R, {age}) & { age: V }`, and `{ ..R, -age }` is `Without(R, {age}) & {..}`. There is no `\` operator and no new token.
*Reason:* `put` and `remove` need a type for "R, except that this field may now be anything". `Without` has an exact rule on the left of `<:` and a sound rule on the right. It is rewritten only by equalities (part 4.3). So no rule depends on whether a type is in an input or an output place, and the coalescing and simplifying in `display.rs` (which make the schemes of generalized bindings, `infer.rs:2108`) cannot make it unsound.

**D4. Keys are label parameters.** A type variable declared on a binding and used as a label (`{ K: V }`) is a label parameter. At each call it is fixed from the known type of its argument, which must be exactly one String literal, or a label parameter of the binding around the call. Union keys, plain `String` keys and keys with no known type are compile errors. A function with label parameters may only be called, not used as a value. The solver never sees a label that is an inference variable.
*Reason:* this is the fallback rule of the task, made general: any library may declare label parameters, and `i/record` uses nothing else. Inference stays principal, because labels come from known types before solving and never from solving.

**D5. Inside its own binding, a label parameter is rigid.** `K` is equal only to itself for `<:` and `&`. For forgetting, `K` may be equal to any label, so `{ ..{ a: Int }, K: V }` keeps `a` only as a possibility: reading `.a` there is the compile error "the key K may be `a`".
*Reason:* both rules stay sound after `K` is replaced by any string (part 5.5).

**D6. `v.f` has three rules.** 13a: when the known type of `v` is made only of custom types, the old rule applies, with `Access::Field`. 13b: otherwise, a fresh `A` and `t <: { f: A }`, with the new `Access::Label`. 13c: tuple parts, unchanged. A known type that mixes custom types and other types is a compile error.
*Reason:* every program that is correct today keeps its meaning and its code. 13b is principal.

**D7. Custom types are not records.** A `User` is never a subtype of `{ name: String }`.
*Reason:* the values have different run-time tags, so the sets of values are disjoint. A record view would break D1, would need a run-time dispatch for every `.f`, and is wrong for a type with several constructors when only some have the field.

**D8. Run time: one `$obj` with tag 14, labels and values interleaved.** Field 5 holds `[label0, value0, label1, value1, …]`, sorted by the bytes of the labels.
*Reason:* the shapes that reach a function are open (width), so one struct type for each shape would need a test for each shape anyway. With this layout the existing generic `$obj` paths of `$value_compare_i32` and `$value_hash_u32` already give the right `==` and `hash`. Only `inspect` needs a branch.

**D9. Syntax.** A `{` is a record when it is `{..}`, when `..` follows it, or when its first part is `label :` with no top-level `=`, `->`, `?` or `|`. A clause head or a binding wins. The same scan is used for a function body, so `(n) { name: n }` gives a record. `{..}` is the empty record and the type of all records. `{}` stays only the empty clause block.
*Reason:* one rule for every `{`. No program that is correct today changes, because a block or clause block never starts with `label :` without one of those four tokens. `{..}` reads back as itself, so the text form of the empty record is i source.

**D10. Out of T3:** record patterns, union keys, a record view of custom types, and the Lisp-surface rows (proposed to M1 in part 13). A type test `x: { … }` is in T3: it takes every record, by head.
*Reason:* patterns touch `pattern.rs` and `coverage.rs`, which T1 owns, and type tests with rule 11 already give a sound way to tell records from other values.

## 3. What a record type means

Types are sets of values.

- A **record** is a value with fields. Each field has a label, which is a String, and a value. No label is in one record twice. The order of the fields has no meaning.
- `{ f1: T1, …, fn: Tn }` is the set of records that have **at least** the fields `f1 … fn`, with the value of `fi` in `Ti`. Other fields may hold anything. It is the same as `{ f1: T1 } & … & { fn: Tn }`.
- `{..}` is the set of all records.
- **Forgetting.** `Without(T, L)` is the set of the values of `T` that are not records, together with every record `x` for which some record `y` in `T` has the same fields as `x` at every label that is not in `L`. Fields at labels in `L` may be there or not, with any values. So `Without(T, L)` holds every value of `T` and more. It is **not** "remove the field": a value of `Without({ a: Int }, {a})` may still have a field `a`.
- `{ ..R, f1: T1, …, -g1, … }` is `Without(R, {f1, …, g1, …}) & { f1: T1, … }`. When there are no `fi`, the second part is `{..}`.
- A record type with a field of type `Never` has no values. It is kept as it is and not turned into `Never`, the same as `(Never, Int)` today. That can only make the solver accept less.

## 4. Types in the checker

### 4.1 Labels

```rust
pub enum Label { Name(Rc<str>), Param(LabelParamId) }
```

- Order: every `Name` before every `Param`; `Name`s by their UTF-8 bytes; `Param`s by id. The order of `Name`s is the order of `$string_compare` at run time (bytes, then length), which is Rust's `str` order.
- `Param` labels exist only inside the binding that declares the label parameter, and in its type scheme. Every call replaces them (part 5).
- Two labels are **the same** when they are equal. They are **different** when both are `Name`s and not equal. Otherwise (at least one is a `Param`, and they are not equal) they **may be equal**.

### 4.2 Forms

In `crates/ilang-check/src/types/mod.rs`:

- `Head::Record`: a new variant with no data, so `Head` stays `Copy`.
- `Ty::Rec(Vec<(Label, Ty)>)`: fields sorted by label, no label twice. `head()` is `Some(Head::Record)`. Every field is covariant.
- `Ty::Without(Box<Ty>, Vec<Label>)`: the labels sorted, none twice, at least one. `head()` is `None`. `is_varlike()` is true, as for `Diff`. The base is covariant.
- `Access::Label(Rc<str>)`: `v.f` on a record. `Access::Field` and `Access::Part` stay as they are.
- A helper `Ty::is_headed()`, true for `Con`, `Fn` and `Rec`. Every place that matches `Ty::Con(..) | Ty::Fn(_)` as "has a type constructor" is checked against it (part 16, step 4).
- `variances(Head::Record, _)` is never called, because `Rec` is not a `Con`. It gets an `unreachable!` arm with a comment.

`map_children`, `extrude`, `freshen`, `bound_vars`, `level_of`, `occurs` and the Coalescer walk every field of a `Rec` and the base of a `Without` with the polarity of the type around them. Field types and labels are kept sorted, so structural `==`, the constraint cache and the quick exit `l == r` (`solve.rs:246`) keep working.

### 4.3 Building types: only exact rewrites

Two smart constructors build every `Rec` and `Without`. They are the only place where these forms are rewritten, and they are used by `solve.rs`, by `Norm` and the Coalescer in `display.rs`, by known types, and after a label parameter is replaced.

`rec(fields)`:

- **N1.** Sort the fields. A label given twice has its two field types put together by `&`. This happens only when two `Param`s are replaced by the same `Name`, or in `&` (4.5). A written type or a literal with a label twice is a compile error before this point (E0540).

`without(T, L)`:

- **E1.** `T` has no records: a `Con`, an `Fn`, or `Never`. The result is `T`.
- **E2.** `without(A | B, L)` is `without(A, L) | without(B, L)`.
- **E3.** `without(Without(T, L1), L2)` is `without(T, L1 ∪ L2)`.
- **E4.** `without(Rec(fs), L)`, when no label of `fs` may be equal to a label of `L`: `Rec(fs` without the fields whose labels are the same as a label of `L)`.
- `without(Top, L)` is `Top`.
- In every other case the form stays: `Without(T, L)`. This is the case for a type variable, an intersection, a difference, and a `Rec` with a label that may be equal to a label of `L`.

Each rewrite is an equality, so it may be used on either side of `<:`, and it stays true when every `Param` is replaced by any `Name`:

- E1 to E3 are equalities of sets.
- E4 is an equality of sets too: the records that agree with a record of `{ fs }` at every label outside `L` are exactly the records of `{ fs }` without the fields at labels of `L`. (One corner: when a removed field type has no values, `{ fs }` has none, and E4 gives a larger type. A larger type is safe for a value made by `put` or `remove`; for a written spread, §4.9.2 defines the meaning by this rule.)
- E4 needs that no label may be equal to a label of `L`. With `Param` labels, the set depends on the strings the `Param`s become, so the form stays and the rules of 4.4 work on it.

So rewriting a scheme in `display::simplify`, in either polarity, never makes a wrong program correct. Where it rewrites a `Without` on the right of `<:`, it can only make a correct program pass that R-without-right would reject.

Apart from the corner of E4, no rewrite makes a type larger. In particular, `Without(a & B, L)` is not split, and `Without(Rec(fs), {K})` with a `Param` `K` is not turned into a smaller record.

### 4.4 Subtyping: the arms of `constrain_in`

The arms go into `constrain_in` (`solve.rs:245`) in this order. Arms that exist today keep their places.

1. (exists) `l == r`, `Never <: _`, `_ <: Top`; the cache.
2. (exists) `Union <: _`; `_ <: Inter`.
3. (exists) the variable arms: `Var <: r` and `l <: Var`, with `extrude` when the levels need it. A `Without` may be stored as an upper or a lower bound of a variable. That is safe, because every bound is checked again by these same arms.
4. **R-split.** `l <: Rec(gs)` with two or more fields, when `l` is not a `Rec`: `l <: Rec([g])` for each field. This is exact, because `{ a: A, b: B }` is `{ a: A } & { b: B }`. It must come before the left-`Inter` arm, so that `a & { x: A } <: { x: A2, y: B }` is tried one field at a time. Each part is smaller, so this ends.
5. **R-without-right.** `l <: Without(U, L)`:
   - when `l` is `Without(B, L2)` with `L2 ⊆ L` (by "the same"): `B <: U`;
   - when `l` is an `Inter` with a member `Without(B, L2)`, `L2 ⊆ L`: `B <: U` for that member;
   - else: `l <: U`.

   This is sound: `Without` is monotone in its base, `Without(B, L2)` is a subset of `Without(B, L)` when `L2 ⊆ L`, `l & m` is a subset of `m`, and `U` is a subset of `Without(U, L)`. It may reject a correct program. That only happens with a spread written in an input place, such as a parameter type `{ ..R, a: Int }`.
6. **R-without-left.** `Without(B, L) <: U` (U is not a variable here, because arm 3 came first):
   - When `U` is var-like, or a union with a var-like member: route it as `route` does (`solve.rs:516`): to the one var-like member, or the error "ambiguous" when there are more.
   - Otherwise let `R` be the record member of `U`, if there is one (after §4.4 merging there is at most one). If a label of `R` is the same as a label of `L`, or may be equal to one, replace `R` by `Never` in `U`, which gives `U↓L`. Then `B <: U↓L`.

   This is exact: a forgotten field can be anything or missing, so a record of `Without(B, L)` is in `R` only when `R` asks nothing about the labels of `L`, and then exactly when the record of `B` it came from is in `R`. When the check fails because `R` was replaced, the failure gets its own message: "the key `K` may be `f`; it is not known here" (E0544) when the labels may be equal, and "the field `f` may be missing here" (E0501) when they are the same.
7. (exists) the left-`Inter` arm (`solve.rs:285`). Its partition counts `Rec` as concrete, next to `Con` and `Fn`: `matches!(m, Ty::Con(..) | Ty::Fn(_) | Ty::Rec(_))`. A `Without` member is tried alone, like a variable.
8. (exists) `Diff <: _`, `Top <: _`, `Both`, `_ <: Union` (route), `_ <: Diff`.
9. **R-width.** `Rec(fs) <: Rec(gs)`: for each `(g, G)` in `gs`, find the field of `fs` whose label is the same as `g`, and constrain `F <: G`. If there is none, fail. The failure is E0544 ("the key `K` may be `g`") when a label of `fs` may be equal to `g`, and E0501 ("the record has no field `g`") when not. With `env` true, fail: records are never services.
10. (exists) `Con <: Con`, `Fn <: Fn`, and the catch-all mismatch. `Con <: Rec` and `Rec <: Con` fall to the catch-all. When the left is a custom type and the right is a record, the message adds the hint of part 11.

### 4.5 `&` and `|`

In `solve.rs` (`inter_in` :744, `union_in` :698, `merge` :801) and in `display.rs` (`Norm::inter` :772, `Norm::union_in` :698):

- `Rec(fs) & Rec(gs)` is `rec(fs ++ gs)`: every label of both; labels that are the same are merged by `&` (N1). Labels that may be equal stay two fields. This is exact for every replacement: when `K` becomes `"a"`, N1 merges `{ K: A, a: B }` into `{ a: A & B }`, which is the same set.
- A `Rec` and a member with another head give `Never`, as today. That is exact: no record is an `Int`.
- `Without` members are not merged with anything. They stay as members of the `Inter`.
- `Rec(fs) | Rec(gs)` is `Rec` of the labels that are the same in both, with their types merged by `|`. This is the MLstruct join, and it makes a larger type, as §4.4 already does for `List(Int) | List(String)`. In a written type the merged type is what the union means (§4.4: members with one type constructor "are put together into one"). So `{ a: Int, b: Int } | { a: String }` **is** `{ a: Int | String }`.
- A `Without` member of a union is not merged.
- In an environment (`env` true) there are never records.

`route` (:516) needs no change: after merging, a union has at most one `Rec` member, and a `Rec` routes to it by head. `A - H` (`diff_in` :877, `diff_left` :544) takes records out by head, with no new code. `known::disjoint` says a record is disjoint from every other head and never from a record.

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
- An `Inter` of `Without(B, L)` and `Rec(fs)` is printed `{ ..B, fs, -l … }`, with `-l` for each label of `L` that is not a label of `fs`. A `Without(B, L)` alone is `{ ..B, -l … }`.
- `display::tree` (:1176 and :1280) gets an explicit `Rec` arm before the arm that shows every other `Con` as a tuple.
- The Coalescer and `Norm` use only the smart constructors of 4.3 for these forms.

Expect changes in the `.out` files of schemes: `(u) { u.name }` used to be an error (E0506), and now it has a type.

### 4.8 Known types (`known.rs`)

- `members` (:21) treats a `Rec` as one member with head `Record`. An `Inter` gives its `Rec` member, as it gives its `Con` member today. A `Without(B, L)` gives the members of `B`, where a record member loses every field whose label is the same as, or may be equal to, a label of `L`. That is a larger type, which is safe for a known type.
- `match_k` (:322) gets a `Rec` arm: when `P` is a `Rec` and `K` has one `Rec` member, each field of `P` is matched with the field of `K` with the same label. Fields with `Param` labels are skipped.
- A record literal has the record of the known types of its fields, with `_` for a field that has none.
- `ksubst` builds types with the smart constructors, so the known type of `record.put({ age: 1 }, "age", "x")` is `{ age: String }`.
- The known type of a parameter `key: K`, where `K` is a label parameter, is `K` (part 5.2).

## 5. Keys: label parameters

### 5.1 The library types

```i
get(K, V): (r: { K: V }, key: K) -> V
put(R, K, V): (r: R & {..}, key: K, value: V) -> { ..R, K: V }
remove(R, K): (r: R & {..}, key: K) -> { ..R, -K }
```

### 5.2 The rules

- **L1. What it is.** A type variable declared on a binding or a declaration (`get(K, V) …`) that is used as a label in a record type in its annotations is a **label parameter**.
- **L2. Where it may be written.** Only as a label, or as the whole written type of a parameter (`key: K`). Every label parameter must be the whole type of at least one parameter of the function type of its binding, so that a call can fix it. The binding must be generalized (its value is a direct value, §6.10). A type parameter of a type definition or of an alias may not be used as a label. Any other use is E0543.
- **L3. Inside its own binding.** The label parameter is rigid: it is `Label::Param(k)`, one string that the caller picks.
  - It is never an inference variable, and it never takes part in a constraint as a type.
  - A parameter `key: K` has the type `String` (T1's String with the value set "all"), so `key` may be used as any String: compared, printed, passed to `record.has`.
  - The known type of `key` is the label parameter `K` (a known type only; known types never take part in constraints, `known.rs:1-9`). A name bound to `key` (`k2 = key`) has the same known type. A String made from `key` by a function (`string.trim(key)`) is a plain String and is not a key.
  - No new type form is needed: T1's String needs no case for keys.
- **L4. At a call.** At a call `f(a1, …)` (also with `|>`), where the scheme of `f` has label parameters:
  1. the key arguments (those whose parameter type is a label parameter) are inferred first, in order;
  2. each label parameter `K` gets a label from the known type of its key argument:
     - a String type with exactly one value `"s"` (a T1 literal type): `K := Name("s")`;
     - a label parameter `J` of a binding around the call: `K := Param(J)`;
     - anything else (no known type, `String`, a union `"a" | "b"`, a cofinite set, `_`): E0541. The call then gets fresh types and nothing more.
     When `K` is the type of more than one parameter, they must all give the same label, or it is E0541.
  3. the scheme is instantiated with these labels (`freshen` takes a label map and rebuilds with the smart constructors), and the parameter type `K` becomes the one-value String type of its label;
  4. the call goes on as every call does (rule 5 of §4.8), with the other arguments.

  So the solver never gets a label that is an inference variable.
- **L5. Not a value.** A name whose scheme has label parameters may only be the callee of a call. Any other use, such as `list.map(xs, record.get)`, is E0542.
- **L6. Recursion.** Inside its own value a binding is not yet generalized (§4.8). A call of itself must give every label parameter its own label (`K := Param(K)`). Otherwise it is E0541, with the note that a function with label parameters calls itself only with its own keys.
- **L7. Principal types.** The labels come from known types, which are found before solving, like the constructors of rule 13a. They never come from solving, so solving does not depend on its own order. Every binding that is accepted has a principal type; programs whose keys are not known are rejected, not guessed. An annotation can always make the key known.

### 5.3 Two label parameters with the same label

`K` and `J` are two `Param`s, so inside the binding `{ K: Int, J: String }` has two fields. When a call fixes both to `"a"`, N1 merges them: `{ a: Int & String }`. That is the right set, because the body was checked for every choice of `K` and `J`, also for `K` = `J`.

So a body that writes `K` and then `J` is checked against "they may be equal":

```i
both(K, J) = (r: {..}, k: K, j: J) -> { K: Int, J: String } {
  r |> record.put(k, 1) |> record.put(j, "s")
}
```

This is rejected with E0544, "the key `J` may be `K`". Without the error, `both({..}, "a", "a").a + 1` would add 1 to `"s"`.

### 5.4 The cases the task names

- **Union keys** (`"a" | "b"`): E0541 for `get`, `put` and `remove`. A later extension may allow them for `get` only, as `r <: { a: V } & { b: V }`. It must never be allowed for `put`.
- **Plain `String` keys:** E0541. A dynamic key belongs in a `Dict`. `record.has` takes a plain String.
- **A key with no known type** (a parameter with no annotation): E0541. The hint says to declare a label parameter.
- **Duplicate keys on `put`:** the field is replaced. That is the defined behavior, and the spread type gives it: `put(put(e, "a", 1), "a", "s")` has `{ a: String }` (E3, E4).
- **Duplicate labels in a literal or a written type:** E0540. Write `{ a: Int & String }`.

### 5.5 Why it is sound

Inside the binding, a type with `Param` labels stands for every type you get by replacing the `Param`s with strings. Every rule must stay true for each such replacement:

- R-width finds a field only by "the same" label, never through "may be equal". After a replacement, the same field is still there.
- `&` keeps fields that may be equal as two fields. After a replacement N1 merges them by `&`, which is the same set.
- `|` keeps only labels that are the same in both. After a replacement the union can only have more common labels, so the type before the replacement is a superset.
- Forgetting (E4, R-without-left) treats every label that may be equal as forgotten. After a replacement fewer labels may be forgotten, so every check that passed before still passes.

These two directions do not contradict each other: for `<:` and `&`, treating `K` as different from `a` only makes types smaller or checks stricter; for forgetting, treating `K` as possibly `a` only forgets more. Both are true for every string `K` may be.

### 5.6 Why `put` is not `R & { K: V }`

With that type, and `put` replacing the field at run time:

```i
bump(R) = (r: R) -> R & { age: String } { record.put(r, "age", "old") }
x = bump({ age: 36 })   // { age: Int & String }, which has no values
x.age + 1               // checks, and at run time adds 1 to "old"
```

With `{ ..R, age: String }`, `x` has the type `{ age: String }`, and the last line is a compile error. A `put` that stops the program on a key that is already there would make `R & { K: V }` sound, but width subtyping hides fields: a `{ name, age }` value passed as `{ name }` would make code stop that looks correct.

### 5.7 The rigid key and forgetting

```i
twist(K) = (r: { a: Int }, k: K) { record.put(r, k, "s") }
// twist : (r: { a: Int }, k: K) -> { ..{ a: Int }, K: String }
twist({ a: 1 }, "a").a + 1    // E0501: the type is { a: String }
twist({ a: 1 }, "b").a + 1    // correct: the type is { a: Int, b: String }
```

The result type of `twist` keeps `Without({ a: Int }, {K})` as a form (E4 does not apply, because `a` may be `K`), so each call works it out exactly after `K` is fixed. Inside the body, `record.put(r, k, "s").a` is E0544, "the key `K` may be `a`".

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
- In the call of `put`, `key` has the known type `K`, so put's label parameter becomes `Param(K)` (L4).
- The body's type `Schema({ ..R2, K: V2 })` is checked against the written `Schema({ ..R, K: V })`. The right side is split; R-without-right takes the `Without` member and gives `R2 <: R`; the `{ K: V2 }` member gives `V2 <: V`.
- At `object |> field("name", string)`: `K := "name"`, `R` gets `{..}`, and the result is `without({..}, {name}) & { name: String }`, which E4 makes `{ name: String }`. The second call gives `{ age: Int, name: String }`.

A test checks this exact program (part 15).

If a later track makes declared type variables rigid, `field` would write `s: Schema(R & {..})`. That is an open question for D-types (part 19).

## 6. Field access `v.f`

`field()` (`infer.rs:1538`) works out the known type `kv` of `v` first, as today. Then:

- **13a. Fields of constructors.** When `v` is known to be made by one constructor (§7.5), or `kv` has members and every member is a custom type: the existing code, unchanged. It records `Access::Field`, and the type of `v` must be a subtype of its known type.
- **13b. Fields of records.** When `v` has no known type, or every member of `kv` is a record type or `_`: make a fresh `A`, constrain `t <: { f: A }`, give `v.f` the type `A`, and record `Access::Label(f)`. The known type of `v.f` is the field `f` of the one `Rec` member of `kv`, when it has one.
- **13c. Tuple parts.** `v.k`: unchanged.
- **Otherwise** (a custom type and a record, or a function, an `Int`, …): E0512 with the message "`.f` reads a field of a record or of a custom type, and this value may be either; write its type". The code generator never sees a mixed target, so the shortcut "the last branch is not tested" in `Lowerer::field` (`lower.rs:1675`) stays correct.

E0506 is no longer given for `.f` (it is still given at `infer.rs:1270`). Every program that is correct today has a known type at each `.f`, so it keeps rule 13a and its code.

A custom value that reaches a record function is a mismatch with a hint:

```i
User { User(name: String) }
name_of = (u) { u.name }        // (u: { name: A }) -> A
name_of(User("bo"))             // E0501
```

> `User` is a custom type, not a record. `.name` here reads a record field because `u` has no known type (§4.8 rule 13b). Write `(u: User) { u.name }` to read the field of the constructor.

## 7. Record expressions and type tests

- **Literal** `{ f1: e1, …, fn: en }`: the parts are worked out left to right (§6.2). Each part has the expected type of the same field of the one `Rec` member of the expected type, if there is one. The type is `rec(fi: ti)`. A label twice is E0540.
- **Spread** `{ ..e, f1: e1, … }`: `e` first, then the parts in order. `te <: {..}`, and the type is `without(te, {f1, …}) & rec(fi: ti)`. `{ ..e }` alone has the type `te & {..}`.
- **Empty record** `{..}`: the type `{..}`.
- **Effects and direct values:** `has_no_effect` (`infer.rs:2340`) treats a literal as it treats a tuple, and a spread as it treats a record update `C(..v, …)`. A literal of direct values is a direct value (§6.10) and is generalized (rule 14).
- **Type tests.** `x: { name: String }` in a clause with more than one clause takes every record, by head. Rule 11 already asks for `B <: T | r - T`, so every record that can come to that place must be in `{ name: String }`. To tell kinds of records apart, use a custom type. `lower.rs` `head_cond` (:2209) maps `Head::Record` to a test of tag 14.
- **No record patterns** in T3. `{ name: n } = r` is a parse error with the hint "read the field: `n = r.name`".

## 8. Syntax

### 8.1 Expressions

A `{` that begins an expression, and the `{` of a function body, are read like this. The parser finds the first part with one scan: it ends at the first `,` or break that is directly inside these braces, or at the matching `}` (found with `close[]`). "Top level" means not inside `( )`, `[ ]`, `{ }` or `<< >>`.

1. `{}` is the empty clause block (syntax.md §6.12).
2. `{..}`, and a `{` followed by `..`, is a record.
3. A `{` whose first part begins with a `name` or a string literal and then `:`, and has no top-level `=`, `->`, `?` or `|`, is a record.
4. Otherwise, the rule of syntax.md §6.1: a clause block or a block. For a function body, it is always a block.

So:

| Source | Reading |
|---|---|
| `{ x: Int -> x }` | clause block (it has `->`) |
| `{ x: Int = 1 ⏎ x }` | block (it has `=`) |
| `{ name: "ada", age: 36 }` | record |
| `{ ..r, age: 37 }` | record |
| `{ "first-name": "ada" }` | record |
| `{..}` | the empty record |
| `(n) { name: n }` | a function that gives a record |
| `(x) { x: Int = 1 ⏎ x }` | a function whose body is a block |
| `(x) { x: Int -> x }` | a function whose body is a block; that block is a compile error, as today |

- `||` and `|>` are other tokens, so they do not count as `|`.
- **The one misreading.** A record whose first field holds a function with a `->` part, such as `{ f: (x) -> Int { x }, n: 0 }`, has a top-level `->` in its first part, so it is read as a clause block. Only the first part is scanned, so the way out is to put another field first, `{ n: 0, f: (x) -> Int { x } }`, or to bind the function to a name first. The error of a clause block whose head is `name: (…)` says this.
- Inside a record, a break is a `,`, as inside `( )`. A `,` at the end is allowed.
- Labels in expressions are `name`s or string literals. An `UpName` label is a compile error ("a label is a name or a string").
- `.f` reaches only labels that are `name`s. Other labels are read with `record.get`.
- `C(..v, f: e)` stays the update of a custom value.

Why a function body uses the same scan instead of a forced error: a block never starts with `label :` without `=`, `->`, `?` or `|`, so no block is lost, and `(n) { name: n }` is the natural way to write a function that gives a record. A forced error would only send the user to `(n) { { name: n } }`.

### 8.2 Types

```
Prim       = FnType | TupleType | NamedType | "_" | "{" Type "}" | RecordType .
RecordType = "{" ".." "}"
           | "{" [ ".." Type "," ] RPart { "," RPart } [ "," ] "}" .
RPart      = Label ":" Type | "-" Label .
Label      = name | string_lit | UpName .
```

- A `{` in a type is a record type when `..` follows it, or a label and then `:`. Otherwise it groups a type, as today.
- `-label` parts are allowed only after a `..` part.
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
| `$record_get(r, label) -> eqref` | Binary search. At each probe it tries `ref.eq` on the label first, then `$string_compare`. A missing label traps with `unreachable`; the types make that impossible, and tests check it. |
| `$record_put(r, label, v) -> eqref` | Copies the array, and puts the field in or replaces it. |
| `$record_with(r, pairs) -> eqref` | A sorted merge of `r` and an already sorted interleaved array, for `{ ..r, … }`: one copy. |
| `$record_remove(r, label) -> eqref` | Copies the array without the field (the same record when it is not there). |
| `$record_has(r, label) -> i32` | Whether the field is there. |
| `$record_keys(r) -> eqref` | The labels as a `List(String)`, in order. |
| `$record_size(r) -> i32` | The number of fields. |
| `$is_record(v) -> i32` | Tag 14, in `emit/mod.rs` next to `$is_ctor` (:16). |

**Label constants.** Lowering makes one immutable global String for each distinct label used in a literal, a spread or a `.f` in the output module. So `.f` passes the same reference that a literal stored, and the `ref.eq` fast path hits.

### 9.4 IR and lowering

The wasm crate already uses "Record" for constructors with fields (`CtorRepr::Record`, `TypeRepr::Record`), so the new names use `Rec`:

- `Value::RecNew(Vec<(LabelConst, Atom)>)`: the atoms are worked out in source order first (§6.2), then put in label order, which is fixed when the program is compiled. Emitted as `(call $record_new (array.new_fixed $values 2n …))`.
- `Value::RecGet(Atom, LabelConst)`: `ExprKind::Field` with `Access::Label`. `Access::Field` and `Access::Part` are unchanged.
- `Value::RecWith(Atom, Vec<(LabelConst, Atom)>)`: a spread.
- `Cond::IsRec(Atom)`: `head_cond` (`lower.rs:2209`) for `Head::Record`, in T1's type-test region.
- The `"ilang:record"` externals: a new arm in `external_op` (`lower.rs:1364`, next to `"ilang:core"` at :1380) that maps each name to its runtime function, as `CORE`/`CORE_FUNCS` do.

### 9.5 inspect

`runtime/values/inspect.rs` gets a branch for tag 14. Task kinds 0 to 3 are all in use (value, byte, parts, list rest; `inspect.rs:291-311`), and the packing `kind | index << 2` (:244) stays as it is:

- kind 0 on a record: an empty array writes `{..}`. Else write `{ `, push the byte `}` and the byte ` ` (kind 1), then push `(2, record, 0)`.
- kind 2 whose value has tag 14 (the parent's tag decides): `index` is always even. Write `, ` when `index > 0`, then the label and `: `, then push `(2, record, index + 2)` and `(0, value at index + 1, 0)`.
- A label that is a `name` (§1.3) is written as it is. Any other label is written in the String text form with `$ib_quote`. A small runtime function `$label_is_name` checks this.

The text form is `{ age: 36, name: "ada" }`, and `{..}` for the empty record. Both read back as the same record.

### 9.6 The host

Records are not in §10.3, so the existing checks reject them at the host boundary (E0318 for `@external`, E0320 for entry points and services). T3 adds a test. `abi.rs` needs no change.

### 9.7 Cost

A `.f` on a record is a binary search with `ref.eq` and string compares: slower than `struct.get` on a custom value, and fine for records of the size of a schema or a JSON object. A `put` copies the array, so building n fields one `put` at a time costs O(n²). `docs/performance.md` gets a note and a micro-benchmark. Shape ids and inline caches can come later on the same layout, as long as `==` and `hash` stay canonical.

## 10. The module `i/record`

`lib/i/record.i`, registered in `crates/ilang-package/src/library.rs` (the list at :12), like `i/core`:

```i
//// Records (§4.9): reading and setting fields by a key that is one known string.

(get, put, remove, has, keys, size)

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
```

- `r |> record.put("age", 37)` works because `|>` is a call.
- `record.get(r, "name")` is the same as `r.name`, and also reads labels that are not names.
- There is no `record.empty`: the empty record is `{..}`.
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
| E0541 | `LABEL_NOT_KNOWN` | a key argument whose known type is not one string | "the key of `record.put` must be one known string, such as `\"age\"`; here it may be any `String`" (or "… may be `\"a\"` or `\"b\"`"). Hint: "declare a label parameter: `f(K) = (r, key: K) …`, or match the key first" |
| E0542 | `LABEL_PARAM_VALUE` | a name with label parameters used other than as a callee | "`record.get` has a label parameter, so it may only be called: write `(r) { record.get(r, \"name\") }`" |
| E0543 | `LABEL_PARAM_PLACE` | a label parameter in a place other than a label or the whole type of a parameter; one that is no parameter's whole type; on a binding that is not generalized; a type parameter used as a label | "the label parameter `K` may only be a label, or the type of a parameter" |
| E0544 | `LABEL_MAY_BE` | a check that needs a rigid label to differ from another label | "the key `K` may be `name`; it is not known here" |
| E0501 | (exists) | a missing field; a custom value given to a record type | "the record has no field `b`"; "`User` is a custom type, not a record. … Write `(u: User) { u.name }` to read the field of the constructor." |
| E0512 | (exists) | `.f` on a value whose known type mixes custom types and other types | "`.name` reads a field of a record or of a custom type, and this value may be either; write its type" |
| E0318, E0320 | (exist) | a record at the host boundary | the existing messages |
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
- "`{ ..R, f: T }` is `R` with the field `f` set to `T`, and `{ ..R, -f }` is `R` with the field `f` forgotten (§4.9.2)."

**§4.1.** Add: "Records (§4.9) are values of one type constructor."

**§4.4.** At the end of the bullet on members with one type constructor, add: "All records count as one type constructor (§4.9.3)."

**§4.5.** Add after rule 7: "8. Records: see §4.9.3."

**§4.8, rule 13** is replaced by:

> 13. **Fields and tuple parts.**
>   - (a) **Fields of constructors.** For `v.f`, when `v` is known to be made by one constructor (§7.5), or every member of the known type of `v` is a custom type, every constructor that may have made `v` (§6.7) must have a field with the label `f`. The type of `v.f` is the union of the types of these fields, with the type arguments put in, and the type of `v` must be a subtype of its known type.
>   - (b) **Fields of records.** When `v` has no known type, or every member of its known type is a record type: `v.f` has the type `a`, with a new type variable `a`, and the type of `v` must be a subtype of `{ f: a }`.
>   - (c) **Tuple parts.** For `v.k`, every member of the known type of `v` must be a tuple with more than k parts. The type of `v.k` is the union of their parts k, and the type of `v` must be a subtype of its known type.
>
>   In every other case it is a compile error.

Add rule 15:

> 15. **Records.** If every `ei : Ti`, then `{ f1: e1, …, fn: en } : { f1: T1, …, fn: Tn }`. If `r : R`, `R <: {..}`, and every `ei : Ti`, then `{ ..r, f1: e1, … } : { ..R, f1: T1, … }`. `{..} : {..}`.

In the known types, add:

- "A record has the record of the known types of its fields, with `_` for a field that has none."
- In **To match**: "When `P` is a record type and `K` has one member that is a record type, each field of `P` is matched with the field of `K` with the same label."
- In the call bullet: "When the type scheme of `f` has label parameters, they are fixed first (§4.9.4)."
- In the expected types: "A field of a record has the matching field of the expected type, when that type has one member, a record type with that label."
- "A parameter `key: K`, where `K` is a label parameter, has the known type `K`."

The example line becomes: "So `(u) { u.name }` has the type `(u: { name: A }) -> A`, and may not be given a `User`. `(u: User) { u.name }` reads the field of the constructor, and with `opt: Option(User)`, `opt |> { Some(u) -> u.name  None -> "" }` is correct."

**§4.9 Records (new).**

> **4.9.1 Values.** A record is a value made of fields. Each field has a label, which is a String, and a value. No label is in a record twice. The order of the fields has no meaning: `{ name: "ada", age: 36 }` and `{ age: 36, name: "ada" }` are the same record.
>
> **4.9.2 Types.** `{ f1: T1, …, fn: Tn }` is the type of the records that have at least the fields `f1 … fn`, where the value of `fi` is in `Ti`. A record may have more fields than its type says. They are kept, and `==`, `core.inspect` and `core.hash` see them. `{ f1: T1, …, fn: Tn }` is the same as `{ f1: T1 } & … & { fn: Tn }`. `{..}` is the type of all records.
>
> `{ ..R, f1: T1, …, -g1, … }` is `R` with the labels `f1 …` and `g1 …` **forgotten**, and `{ f1: T1, … }`. A record type `R` with the label `f` forgotten is the type of the records that have the same fields as a record of `R` at every other label; at `f` they may have any value, or no field. So `{ ..{ age: Int, name: String }, age: String }` is `{ age: String, name: String }`. The values of `R` that are not records stay as they are, so `{ ..Int, a: Int }` has no values.
>
> **4.9.3 Rules.** Records are one type constructor. So a record is never a value of a custom type, a tuple, a function type, or a built-in type, and the rules of §4.4 for one type constructor hold:
>
> - `{ fs } <: { gs }` when every label of `gs` is in `fs`, and the field type in `fs` is a subtype of the field type in `gs` (width).
> - `{ fs } & { gs }` has the labels of both. A label in both has the `&` of its two types.
> - `{ fs } | { gs }` is put together into the record of the labels in both, with the `|` of their types. So `{ a: Int, b: Int } | { a: String }` is `{ a: Int | String }`. To tell kinds of records apart, use a custom type.
> - A type test `x: { … }` in a clause takes every record (§4.8 rule 11). So `x: { a: Int }` on a value of type `{ a: Int } | { a: String }` is a compile error.
>
> Why setting a field forgets it first: if `record.put(r, "age", "old")` had the type `R & { age: String }`, then for `r = { age: 36 }` it would be `{ age: Int & String }`, which has no values, and `.age + 1` would be correct and add 1 to `"old"`. With `{ ..R, age: String }` it is `{ age: String }`, and `.age + 1` is a compile error.
>
> **4.9.4 Label parameters.** A type variable declared on a binding or a declaration that is used as a label in a record type, as `K` in `get(K, V): (r: { K: V }, key: K) -> V`, is a **label parameter**. It may only be used as a label, or as the whole type of a parameter, and every label parameter must be the whole type of some parameter of the function. The binding must be generalized (§4.8 rule 14).
>
> At each call of a name whose type has label parameters, each label parameter takes the one string of the known type of its argument: a String type with exactly one value, or a label parameter of the binding that holds the call. If that known type is not one string, it is a compile error. So `record.get(r, "name")` is correct, and `record.get(r, s)` with `s: String` is not. A name with label parameters may only be called. It may not be used as a value. Inside its own value, a binding calls itself only with its own label parameters.
>
> Inside its binding, a label parameter `K` is one string that is not known. A parameter `key: K` there has the type `String`, and the known type `K`. `{ K: A }` and `{ f: B }` have different fields, and `{ ..R, K: V }` forgets every field of `R` that may be `K`. Where a check needs to know that `K` is not `f`, it is a compile error: "the key `K` may be `f`".
>
> **4.9.5 The module `i/record`.** See §14.

**§6.1 Grammar.** Add `| Record` to `Primary`, with:

```
Record      = "{" ".." "}"
            | "{" ".." Expr { "," RField } [ "," ] "}"
            | "{" RField { "," RField } [ "," ] "}" .
RField      = ( name | string_lit ) ":" Expr .
```

Replace the bullet "A `{` followed by a label `name :` … is kept for records" by the reading rule of part 8.1: "A `{` that begins an expression or a function body is a record when it is `{..}`, when `..` follows it, or when its first part begins with a `name` or a string and then `:`, and has no `=`, `->`, `?` or `|` outside brackets. The first part ends at the first `,` or break directly inside the braces. A function body that is not a record is a block. So a record whose first field holds a function with a `->` part is read as a clause block: put another field first, or bind the function to a name first."

**§6.2.** Add: "In a record `{ f1: e1, … }` the parts are done in the order written. In `{ ..e, f1: e1, … }`, `e` is done first."

**§6.10.** In the list of direct values, add "a record whose parts are direct values".

**§6.13 Records (new).**

> `{ f1: e1, …, fn: en }` makes a record (§4.9). A label may not be given twice. `{ ..e, f1: e1, … }` makes the record `e` with the given fields added or put in place of its fields; `e` must be a record. `{..}` is the record with no fields; `{}` is the clause block with no clauses (§6.12).
>
> `v.f` reads the field `f` (§4.8 rule 13). A field whose label is not a `name` is read with `record.get`. Fields are set and removed with `record.put` and `record.remove` (§14):
>
> ```i
> ada = { name: "ada", age: 36 }
> older = { ..ada, age: ada.age + 1 }
> r = {..} |> record.put("name", "ada") |> record.put("age", 36)
> ```

**§7.1.** Add: "A type test whose type is a record type takes every record (§4.9.3). There are no record patterns."

**§7.4.** Add: "All records count as one type constructor."

**§10.3.** Add: "Records may not go to the host."

**§11.1.** Add "records" to the list of values.

**§11.4.** Add: "Records are the same when they have the same labels and the values with the same label are the same. So `{ a: 1, b: 2 } == { b: 2, a: 1 }`, and `{ a: 1 } != { a: 1, b: 2 }`, also where both have the type `{ a: Int }`. A record is never the same as a value that is not a record."

**§11.8.** Add a row: "Record | `{ `, then for each field in the order of the bytes of its label: the label (as it is when it is a `name`, and in the String text form when not), `: `, and the text form of the value, with `, ` between fields, then ` }`. The record with no fields is `{..}`. So `{ age: 36, name: \"ada\" }`."

**§12.** Add `i/record` (§14) to the list of modules. IO owns the sentence that counts the modules, so this edit is made with IO.

**§14 The module `i/record` (new).** A table with the six functions of part 10, their types, and one line each, in the form of §13.

## 13. Other documents

These are applied by T3, after G3. They are not part of this commit.

- **syntax.md.** In the summary, "`{ name: e, … }` is kept for records (stream T3)" becomes "`{ name: e, … }` is a record (`docs/design/records.md`)". In §6.1, the "kept for records" bullet becomes the reading rule of part 8.1. In §1.7, add the record line.
- **meta.md §11 question 2.** The answer of part 8.2: a one-part `{ }` in a type is a group, unless it is `{..}`, begins with `..`, or is `label: T`.
- **Lisp surface (proposed to M1 and D-meta; M1 decides).** In a `{ }` datum, a first part `(: label e)` or `..` makes a record: `{ (: name e) (: age e) }`, `{ .. r (: age e) }`, `{ .. }`, and in types `{ (: name T) }`, `{ .. R (: K V) (- age) }`. The core-form tables of meta.md §2.3 would get these rows.

## 14. Files to change

Line numbers are at 1865366.

**`crates/ilang-syntax`**
- `ast.rs`: `ExprKind::Record`, `TypeKind::Record`, `RecordField`, `LabelName`.
- `parser.rs`: the record scan, used by `primary` (:766) and by `function` (:858) for the body `{`; `type_prim` (:1228), whose `T::LBrace` arm today always groups; a sequence reader in which a break is a `,`; the hints for `Person { … }` (`type_def`, :474) and for `{ name: n } = r`.
- `format.rs`, `print.rs`, parser tests, `tests/format.rs`.

**`crates/ilang-check`**
- `lib.rs` (:73-101): codes E0540 to E0544.
- `hir.rs`, `resolve.rs`: record nodes; labels are not names; `UpName` labels resolve to declared type variables (label parameters) or are an error; E0540, E0543.
- `types/mod.rs`: `Label`, `LabelParamId`, `Head::Record`, `Ty::Rec`, `Ty::Without`, the smart constructors `rec` and `without`, `is_headed`, `Access::Label`; the scheme gets the list of its label parameters and the index of the parameter of each.
- `types/solve.rs`: the arms of 4.4; `inter_in`, `union_in`, `merge` (4.5); `add_bound`, `reach`, `compare`, `needs_join` (4.6); `map_children` (:642), `extrude` (:577), `freshen` (:618, with a label map), `bound_vars` (:1207), `level_of` (:203); `FailKind::LabelMayBe`.
- `types/display.rs`: the Coalescer (:503), `Norm` (:632, `union_in` :698, `inter` :772), `tree` (:1176, :1280), and the shown errors. `types/shown.rs`: `ShownType::Record`.
- `types/known.rs`: `members` (:21), `match_k` (:322), `disjoint`, record literals.
- `types/infer.rs`: `field()` (:1538) in three rules; record literals and spreads; label parameters in `call()` (:1260) and `instantiate` (:1000); E0541, E0542; `has_no_effect` (:2340).
- `types/pattern.rs`, `types/coverage.rs`: only the `Head::Record` arms that the audit finds, after T1's changes there.
- `query.rs`: `Access::Label` has no definition target; hover shows the field type; completion after `.` lists the labels of the one `Rec` member of the known type.

**`crates/ilang-wasm`**
- `ir.rs`: `Value::RecNew`, `Value::RecGet`, `Value::RecWith`, `Cond::IsRec`.
- `lower.rs`: `ExprKind::Field` with `Access::Label` (:1502), literals and spreads, label constants, `head_cond` (:2209), the `"ilang:record"` arm in `external_op` (:1364).
- `emit/mod.rs`: `$is_record` (next to :16), the label globals, the new values.
- `runtime/mod.rs`: tag 14 in the table (:37-52). `runtime/values.rs` and the new `runtime/values/record.rs`. `runtime/values/inspect.rs`: the branch of 9.5.

**Library, package, docs**
- `lib/i/record.i` (new); `crates/ilang-package/src/library.rs` (:12); the library tests.
- `docs/spec.md` (part 12), `docs/design/syntax.md` and `docs/design/meta.md` (part 13), `docs/performance.md` (9.7).

## 15. Tests

### 15.1 Spec tests (`tests/spec/4.9/`, `tests/spec/6.13/`, `tests/spec/11/`)

Each has a `// spec:` and an `// expect:` header, as in `tests/spec/4.8`.

Correct programs (`// expect: pass`):
- `literal_type.i`: the scheme `{ age: Int, name: String }`.
- `width.i`: `(u) { u.name }` used on records of two shapes.
- `field_principal.i`: `(u) { u.name }` is `(u: { name: A }) -> A`.
- `inter_fields.i`: `(x) { x.a + x.b }` is `(x: { a: Int, b: Int }) -> Int`.
- `many_reads.i`: `(x) { (x.a, x.b, x.c, x.a) }` gives `{ a: A, b: B, c: C }`; a second bound on `a` is checked against the lower bound.
- `union_join.i`: two branches give `{ a: Int | String }`, and `.a` works.
- `spread_override.i`: `{ ..{ a: 1 }, a: "s" }.a` is a String.
- `put_get.i`, `put_override_type.i`: `record.put({ age: 1 }, "age", "x")` is `{ age: String }`.
- `put_remove.i`: `record.remove({ a: 1, b: 2 }, "a")` is `{ b: Int }`.
- `twist_call.i`: part 5.7's `twist({ a: 1 }, "b").a + 1`.
- `schema.i`: the program of part 5.8, exactly as written, with the scheme of `field` and the type of `user`.
- `label_param_pass.i`: a label parameter passed through two levels of functions.
- `string_labels.i`: `record.get({ "first-name": "ada" }, "first-name")`.
- `type_test_record.i`: `x: {..}` takes the records out of `Int | { a: Int }`.
- `nominal_field_unchanged.i`: rule 13a with known types and with §7.5 narrowing.
- `function_body_record.i`: `(n) { name: n }`.
- `spread_upper_bound.i`: a spread reaches an upper bound through a variable: a parameter `p: { ..R, a: Int }` in an input place, and a variable bounded above by a `Without`.

Programs that must be rejected (the refuter's list), each with its code:
- `bump_unsound.i` (E0501): part 5.6's `bump` with `put`.
- `twist_unsound.i` (E0501): `twist({ a: 1 }, "a").a + 1`.
- `twist_body.i` (E0544): `record.put(r, k, "s").a` inside `twist`.
- `two_keys.i` (E0544): part 5.3's `both`.
- `put_then_read_old.i` (E0501): `record.put({ a: 1 }, "a", "s").a + 1`.
- `union_key.i` (E0541): a key with the known type `"a" | "b"`, made by a clause block.
- `string_key.i` (E0541): a key of type `String` from a parameter.
- `unknown_key.i` (E0541): a key with no known type.
- `recursive_key.i` (E0541): a function with a label parameter that calls itself with another key.
- `label_fn_value.i` (E0542): `list.map(xs, record.get)`.
- `label_param_place.i` (E0543): `-> K`; a label parameter that no parameter has as its whole type.
- `missing_field.i` (E0501): `{ a: 1 }.b`.
- `record_not_never.i` (E0501): `{ a: 1 }` where an `Int` is needed; also `{ a: 1 } & { b: 1 }` built by inference and given where an `Int` is needed. This guards against a record becoming `Never`.
- `nominal_not_record.i` (E0501): a `User` given to `(u) { u.name }`, with the hint.
- `mixed_target.i` (E0512): `.name` on `User | { name: String }`.
- `union_lost_field.i` (E0501): `.b` on a match whose branches give `{ a: 1, b: 2 }` and `{ a: "x" }`.
- `type_test_record.i` (E0501): `x: { a: Int }` on `{ a: Int } | { a: String }`.
- `duplicate_label.i` (E0540): `{ a: 1, a: 2 }` and `{ a: Int, a: String }`.
- `record_to_host.i` (E0318, E0320).
- `empty_braces.i` (E0501): `{}` where a record is needed, with the hint.

### 15.2 Checker unit tests

- Each rewrite E1 to E4 and N1; that a `Rec` with a field that may be `K` is not rewritten.
- R-split, R-without-left, R-without-right, R-width; the left-`Inter` arm with a `Rec` member.
- `&` and `|` of records, with `Param` labels.
- **Stable under replacement:** check a body with `Param` labels, replace every `Param` by each label of the body and by a new one, check again, and compare: every check that passed must pass.
- **No over-approximation:** a debug assertion in `Norm` and the Coalescer that a `Without` is rewritten only by `without()`; a test that runs `display::simplify` on schemes with `Without` in input and output places, and checks that no call of the reject list of 15.1 becomes correct.
- `add_bound` with many `.f` on one variable: two fields, then a lower bound with only one; then two bounds on one label and a lower bound that fits only one of them. Both must fail.

### 15.3 Parser tests

`{ x: Int -> x }` is a clause block. `{ x: Int = 1 ⏎ x }` is a block. `{ x: 1, y: 2 }`, `{ ..r, x: 1 }`, `{..}` and a record over several lines are records. `{ "a b": 1 }` is a record. `{ n: 0, f: (x) -> Int { x } }` is a record. `(n) { name: n }` is a function with a record body. `(x) { x: Int = 1 ⏎ x }` has a block body. `(x) { x: Int -> x }` has a block body and is an error. `{}` is a clause block. In types: `{ x: T }` is a record type, `{ T }` and `{ x }` are groups, `{..}` is all records, `{ ..R, K: V, -a }` is a record type, and a break inside a record type is not `TypeGroupBreak`. `Person { name: String }` gives its hint. The formatter round-trips each of them.

### 15.4 Codegen tests (`crates/ilang-check/tests/codegen.rs`, `crates/ilang-wasm/tests/codegen.rs`)

`Access::Label` for `r.name` on a record; `Access::Field` still for `p.name` on a custom type; `Access::Part` for `t.1`. A literal is one `$record_new` with sorted labels. The WAT of every program with no records changes only in the runtime part (checked with `scripts/wat-diff`).

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

## 16. Checklist for track T3

In order, after A1 and T1 have landed, T2 has merged, and S3 is approved.

1. **Spec text (gate G3).** Part 12 goes to the user, written on top of the spec after T1 and T2. Check that §4.9, §6.13 and §14 are still free, with IO and M2 (part 19).
2. **Runtime order.** Check that `$string_compare` orders exactly as Rust `str` does, with a test. (At 1865366 it does: bytes, then length.)
3. **Soundness tests first.** Write the reject list of 15.1 and the unit tests of 15.2 as failing tests.
4. **Types core.** `Label`, `Head::Record`, `Ty::Rec`, `Ty::Without`, `rec()`, `without()`, `is_headed`, `Access::Label`, with unit tests. Then the audit: `rg -n 'Ty::Con\(\.\.\) \| Ty::Fn\(_\)' crates/ilang-check/src` must have no hits left that do not also handle `Rec`. About 127 sites at 1865366 (solve 24, display 39, known 18, infer 32, pattern 8, coverage 2, mod 4); `reach` and the `new` list of `add_bound` (`solve.rs:397`, :438) first.
5. **Solver.** The arms of 4.4, `&` and `|` of 4.5, the bounds of 4.6. Run the unit tests.
6. **Display and known types.** `display.rs`, `shown.rs`, `known.rs`. Accept the changed scheme outputs with `UPDATE=1`, and read each change.
7. **Syntax.** AST, the scan, types, the printer and the formatter, parser tests. Remove the "kept for records" error.
8. **HIR and resolve.** Labels, label parameters, E0540, E0543.
9. **Inference.** Literals and spreads, `field()` in three rules, `has_no_effect`, then label parameters in `call()` (E0541, E0542, L6).
10. **Run time.** Tag 14, `record.rs`, the inspect branch, IR, lowering, emit, `external_op`. Gate: `scripts/wat-diff` shows no change in the generated code of programs with no records.
11. **Library.** `lib/i/record.i`, its registration, and the library tests (names, labels).
12. **Tests.** 15.1 to 15.5 at `-O0` and `-O2`, the spec test suite, and the full dev-verify.
13. **Docs.** `docs/performance.md` (9.7, with the micro-benchmark), syntax.md and meta.md (part 13), and `query.rs` hover and completion.

## 17. Risks

- **The audit is large.** About 127 places treat `Ty::Con(..) | Ty::Fn(_)` as "has a type constructor". A missed one makes `Rec` fall into a catch-all: usually a wrong mismatch, but in `reach` or `add_bound` a bound could be dropped with no error. The `is_headed` helper, the grep gate of step 4, and the reject tests guard this.
- **Unions of records lose fields.** `{ kind: "circle", r: Float } | { kind: "square", side: Float }` becomes `{ kind: "circle" | "square" }`. Users who know TypeScript may expect a tagged union. Custom types do that in i; the spec says so. Exact unions of records would break "one member per head" everywhere.
- **Label parameters are fixed from known types.** A computed key is rejected, and a label-generic function cannot be passed as a value. This is the price of keeping label variables out of the solver.
- **Forgetting with a rigid key loses fields.** In generic code, `{ ..R, K: V }` keeps no field of `R` that may be `K`. That is sound, and E0544 says why, but it may surprise users.
- **R-without-right is not complete.** A spread written in an input place is checked with `X <: U`, which may reject a correct call. It never accepts a wrong one.
- **Flexible declared variables.** The schema example relies on `R` getting the bound `{..}` by inference. If D-types makes declared variables rigid, such code must write `Schema(R & {..})`.
- **The misreading.** A record whose first field holds a function with a `->` part is read as a clause block. The error says to put another field first.
- **Changed outputs.** `(u) { u.name }` without a type now has a type instead of E0506, so expected outputs and messages change, and a custom value given to it fails at the call.
- **Run-time cost.** `.f` on a record is a binary search, and `put` copies. Fine for small records; shapes and caches are later work.
- **inspect.** The new branch shares kind 2 with tuples and custom values. A mistake there changes the text form of other values; the existing inspect tests must not change.
- **Order of work.** T3 depends on T1 (literal types and their known types), A1 (the parser) and T2 (`{ x }` in types). Its spec text must be put on top of theirs. If T1 represents literal types differently, L4 must follow it.

## 18. Rejected alternatives

- **One head for each label** (`Head::Field(label)`): breaks "different heads are disjoint" in `inter_in`, `Norm::inter`, `-`, `route` and `disjoint`. The left `&` would build `Never`, and `Never` fits everywhere.
- **`put` as `R & { K: V }`:** unsound (5.6).
- **Over-approximating spread rules** (a spread with a variable base only in output places, and rewrites that make a type larger): their soundness would rest on the polarity used by the Coalescer, which also makes the schemes of generalized bindings. `Without` with exact rewrites removes that dependence.
- **A `\` operator** (`R \ k`): a new token and a second form for one idea. `{ ..R, -k }` says the same inside the spread form.
- **A separate key type** (`Ty::Key`): one more case in every audit and in T1's String. The known type `K` and the type `String` are enough.
- **Label variables in the solver:** heads would depend on solving, and principal types would be lost.
- **Custom types as subtypes of their record view:** D7.
- **One struct for each record shape:** shapes are made at run time by `put`, and `.f` under width would need a test for each shape.
- **Widening the inspect packing to `<< 3`:** not needed, because kind 2 can look at the parent's tag.
- **`{}` as the empty record or the type of all records:** `{}` is the empty clause block, and the text form would not read back.
- **`{ f: { (x) -> Int { x } } }` as the way out of the misreading:** the inner `{ }` is itself a clause block by syntax.md §6.1 (its first thing is `(x)` followed by `->`), so it does not help. Putting another field first does.
- **Record patterns in T3:** they touch T1's `pattern.rs` and `coverage.rs`. Type tests and `.f` cover the need for now.

## 19. Open questions

1. **Numbering.** Are §4.9, §6.13 and §14 still free after T1, T2, IO and M2? If §14 is taken, `i/record` takes the next free number.
2. **D-types.** Do declared type variables stay flexible (part 5.8)? If not, is `Schema(R & {..})` the form library authors should write?
3. **Union keys for `get`.** Allow `record.get(r, k)` with `k: "a" | "b"` as `r <: { a: V } & { b: V }` in a later track?
4. **Record patterns.** `{ name: n }` as a pattern, after T1's pattern work: a later track, or never?
5. **Lisp surface.** Does M1 take the rows of part 13?
6. **Shapes.** Should `-O2` add shape ids and inline caches for `.f` later, keeping `==` and `hash` canonical?
