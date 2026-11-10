# Value types and alias patterns (S6)

Status: draft for review (study S6 of the redesign plan). When it is accepted at gate G2/G3, track T2 puts the spec text of part 7 into `docs/spec.md` and makes the changes of parts 8 to 10. Code facts are from commit c69515c. Line numbers are at that commit; A1, A3 and T1 move many of them.

T2 lands after A1 and A3 (the new syntax in the spec and in the parser, `ast::TypeKind::Value`, UpName type variables) and is merged after T1 (plan: T1 → T2 → T3 on the hot files). The spec text below is written in the new syntax, on top of syntax.md's replacement text.

## 1. The problem

syntax.md keeps a lowercase name in a type for "the type of a value", and makes it a compile error until T2. meta.md keeps `m(…)` and `a.m(…)` in a type for macro calls. The plan names the main use: a library builds a value, and code takes its type apart without writing it again. In records.md 5.9:

```i
user = object |> field("name", string) |> field("age", int)
Codec(User, _) = user        // User is { age: Int, name: String }
```

So T2 must say:

- which type `x` in a type is, in an annotation and in a type definition, for generalized names, for names that are not generalized, for locals and for `m.x`;
- how a type can depend on the inferred type of a value, when the checker resolves types before values (`resolve.rs`), builds groups only from expressions (`order.rs`), and expands aliases lazily (`infer.rs` `conv`, `known.rs` `kconv`);
- how `Codec(User, _) = user` (a pattern) is told from `Pair(A) = (A, A)` (a generic alias), in a way that an import can never change;
- what `_` means on the left of an alias pattern;
- what this does to §2.4, §2.5, §3, §4.3, §4.7, §4.8, to queries, to the package interface, to records, literal types and macros.

The design must be sound first. It must keep principal types, add no keyword, and treat no library in a special way.

## 2. The decisions

Each decision has its reason.

1. **`x`, `a.x` and `{ x }` in a type are the type of the value.** The name is found as an expression name is found at that place (§5, the same covering rules, the same hygiene marks). *Reason:* no keyword, and one lookup rule for names. `{ x }` is `x` because a `{ }` with one part only groups (records.md 8.2, meta.md).
2. **It is the type, never the known type.** *Reason:* a known type may have fewer values than may come (literal-types decision 6: after `x = "a" = s`, `x` has the type `String` and the known type `"a"`). Decision 7 of literal-types forbids a known type that is smaller than the type.
3. **There are two readings.** In an annotation, `x` is an *instance*, like the expression `x` (§4.8 rule 3). In a type definition, `x` must be *closed*: no type variable and no `_`. *Reason:* an annotation is only another source of constraints, as `_` holes are, so it is sound and keeps principal types. A type definition stands for one type; with variables in it, two uses could pick two types for one part (the hole `Box { Box(id) }`: build with `Int`, take apart as `String`).
4. **A top-level binding that is not generalized is read through its fixed type.** This is its type at the end of its group, with each variable replaced by the types that came to it or went from it (part 3.3). A variable at both kinds of places is *not fixed*, and reading it is an error. *Reason:* the uses of a binding only put its type on the left of a constraint, so these parts cannot change later. Without this rule, `y: user = c` would add lower bounds to the decode variable of `user`; one annotation would change another binding's type, and `Codec(User, _) = user` would depend on the order of the groups.
5. **A type use of a value is a new kind of edge, in the group graph only.** It is never a need and never changes the init order (§2.5). A cycle with a type edge in it is an error, E0530. *Reason:* a type reads no value at run time. Since every such cycle is an error, the groups of a correct program are the same as today (part 3.4).
6. **An item is an alias pattern exactly when its right side is a value type.** The right side alone decides it, never the names in scope. *Reason:* an import can then turn a correct program into an error, but never give it a new meaning. Nothing is lost: a generic alias whose whole body is one value type is always an error (its parameters are unused, E0221, and a value type in a definition must be closed).
7. **Inside a pattern, a plain `UpName` always defines a new type, `_` skips its part.** *Reason:* the same as a lowercase name in a value pattern, which always binds. If `User` could mean an existing type when one is in scope, an import would change the meaning.
8. **The head of a pattern may be an alias that stands for one named type.** *Reason:* a library may rename `Codec`; a user should still be able to take its values apart. The alias body is put in, and its parts become parts of the pattern (part 3.5).
9. **Value types are not allowed in constructor fields, external declarations, host types and environment types** (E0531), also through an alias. *Reason:* these are trusted, or converted before any group (`infer.rs` around :157-215, `variances`), or checked eagerly in resolve (`env_type` :1445, `host_part` :1568), or read by field access `v.f` with no syntactic use to order by. Custom types stay complete before inference, as meta.md §4.4 promises.
10. **A pattern is matched, and a definition with a value type is read, once, at the end of the group of its source.** *Reason:* the order of errors and `Typed.alias_types` must not depend on who reads a binder first, and a pattern nobody reads must still be checked.
11. **`a.x` in a type is always a member of a module, never a field.** When a local name covers the module `a`, it is an error (E0207, "a type may name a value, not a field of a value; write the field's type"). *Reason:* it never silently changes meaning, and a field type would need the known type, which decision 2 rules out.
12. **`x(` and `a.m(` in a type stay macro calls** (meta.md). `f(A)` on a value name is never type application. *Reason:* one meaning per form.
13. **Each binder of a pattern is an ordinary `TypeDef` whose body is an alias of a new `hir::TypeKind::Of`.** There is no new `TypeBody` variant. *Reason:* exports, E0208, query targets and meta `Definitions` keep working unchanged.
14. **T2 takes the codes E0530–E0537.** T1 keeps E0523–E0529, T3 keeps E0540–E0549. *Reason:* the streams merge in different orders and must not collide. records.md line 715 ("T1 and T2 number their new codes from E0523 upward") is updated when T2 lands (checklist step 0).
15. **No cost for programs without value types.** No new edges, no new passes, no new solver variables, no allocation beyond empty vectors. *Reason:* std and most programs never use the feature.

## 3. Exact rules

### 3.1 Form and lookup

- **V1.** In a written type, a lowercase `name` with no `(` after it, or `a.x` with a lowercase `x` and no `(` after it, is a **value type**. `{ x }` and `{ a.x }` are the same as `x` and `a.x`. `{ x: T }` is a record type (T3). With `(` after it, it is a `TypeCall` (macros).
- **V2.** `x` is looked up as a value name at the place of the type: locals, then top-level names of the module, then imported names, then built-in values (§2.3, §5). A name that is a module is E0205. An unknown name is E0201.
- **V3.** `a.x` is the member `x` of the module `a` (`member()`, resolve.rs :1853). It must be exported (E0204, E0203). When a local value covers `a`, it is E0207 with the message of decision 11. In particular `(p: User, q: p.name)` stays an error.
- **V4. Own scope.** A value type may not name:
  - a name bound by the same pattern (a binding pattern, a parameter pattern, a clause pattern): `(x, y: x)`, `x: { x } = …`, `{ (a, b: a) -> … }`;
  - a parameter of the same function, in that function's parameter types or its `->`, `?` and `!` parts: `(x) -> x { … }`;
  - a name inside its own value: a local `f = (g: f) { g }`, or a top-level `f = (g: f) { g }`.

  Each is **E0536**. The message names the outer binding that the name would otherwise have found, if there is one: "`x` is bound by this parameter list, so a type here may not name it; without this rule it would name the top-level `x` at line 3". *Reason:* at c69515c, `function()` (resolve.rs :2133) resolves parameter patterns, with their types, before it pushes the parameter frame, and resolves the `->` part before it too; a block binding's type is resolved before its name is bound. So without V4 these names would silently find an outer `x`. Left-to-right scoping (`(a, b: a) -> a`) is a later extension that cannot change meaning, because today it is an error (open question 3).
- **V5.** A value type that names a binding with label parameters is E0542 (records L5), and one that names a key parameter is E0543 (records L2a). T3 checks both in resolve; T2 gives it the names (part 3.9).

### 3.2 The two readings

Every value type in HIR is `Of { value, site, part }` (part 8). Its **site** says which reading it has.

- **Annotation reading.** Any type written in a binding: the type of its pattern, parameter types, `->`, `?` and `!` parts, clause patterns and type tests, and the same places in inner bindings. (Environment types, `Stmt::Env` and env params may not hold a value type, decision 9.) Here `Of(x)` is:
  - **A1.** `x` is top-level (this module or another), a declaration, or a built-in: the **read scheme** `read[x]` (part 3.3). Each general variable becomes a new variable at the current level, one per variable for this place (all occurrences of one general variable at this place share it). Each `Top` part becomes a new variable, one per occurrence, as each written `_` does. A part in the **unfixed set** gives **E0534**.
  - **A2.** `x` is a generalized local whose binding is finished: `instantiate(scheme)`, then each variable of a finished top-level group found in the result is replaced by its fixed form, which may give E0534.
  - **A3.** `x` is a parameter, a clause name, or a local that is not generalized, all inside the top-level binding being typed: its holder type, **shared**. So `(xs) { acc: xs = [] … }` gives `acc` and `xs` one type. The annotation puts that type on the right of a constraint (`List(Never) <: τxs`), so the value of `acc` must fit in the type of `xs`, and every use of `acc` asks the same of `xs`. This is sound: the holder is a variable of the binding being typed, so it is not frozen. A constraint on it only moves along the bounds it already has. A frozen variable is linked to it only in the frozen variable's own direction (a `Lower` variable γ is linked by γ <: α because γ was used on the left; an `Upper` variable β by α <: β), so new bounds on α reach γ only as upper bounds and β only as lower bounds, which the invariant of part 3.3 allows. A key parameter is never read this way: it is E0543 (V5).
- **Definition reading.** The body of an alias, or the source of an alias pattern. `Of(x)` is `read[x]`, and every part that is used must be **closed**: no general variable, no `Top`, no part in the unfixed set. Otherwise **E0533** (a general variable or `Top`) or **E0534** (unfixed). The message says which part is open, shows it with the written name of the alias, never with `_`, and says "write the type of `x`". For an alias pattern, only the parts below a binder are used (part 3.5). The result is closed, so it is stored once and each use is a clone (part 3.6).

### 3.3 Fixed types (solver terms)

- **F1. When.** At the end of `Checker::group` (infer.rs :233), after the holders are linked and after `generalize` (:2108), when `program.has_value_types` is true. It runs for every group of every module, because a variable of an earlier group may be reached later.
- **F2. Which variables.** At the start of `group`, record `first_var`, the number of solver variables. A variable `v` **belongs to G** when `v.index >= first_var` and `v` was not generalized (its level is at most the level of the group's outer scope; 0 for a top-level group). Variables with a smaller index belong to an earlier group and already have a frozen entry or were never reached by a frozen group; they are never recomputed.
- **F3. Polarity.** Walk the raw bounds, not the simplified type. Start from the type of every binding of G, as a positive place. A function parameter flips the polarity; an invariant constructor argument is both; a contravariant one flips; the result, `?` and `!` parts keep it. At a variable `v` reached at a positive place, mark `v` positive and walk its lower bounds positively; at a negative place, mark it negative and walk its upper bounds negatively. A variable that belongs to an earlier group (already in `frozen`) stops the walk. A visited set per (variable, polarity) stops cycles.
- **F4. Fixed forms.** For each variable `v` of G that the walk marked:
  - only positive: `Fixed::Lower(L)`, where `L` is the union of its lower bounds, with each variable inside replaced by its own fixed form (recursively, by the same walk); `Never` when there are none;
  - only negative: `Fixed::Upper(U)`, the intersection of its upper bounds, replaced the same way; `Top` when there are none;
  - both: `v` goes into `unfixed: HashSet<V>`;
  - a fixed form that would contain itself (a recursive bound): `v` goes into `unfixed` (the message notes "a type that has itself inside it").
- **F5. The read scheme.** For each binding `x` of G that is a **read target** (`program.read_targets`, part 3.6): `read[x]` is its scheme with each frozen variable replaced by its fixed form. General variables stay as general variables. Unfixed variables stay as variables and are found through `unfixed` when read. The solver `Ty` gets no new variant.
- **F6. Joint simplification.** The fixed forms may then be made simpler with `display::simplify` for showing only. The soundness decision is made by F3–F4 on raw bounds, never by what simplify removes.

**The invariant (stated in the spec, §4.7, as a rule later features must keep).** After a group ends, a variable that it froze as `Lower` only gains upper bounds, and one frozen as `Upper` only gains lower bounds. Uses of a binding only put its type on the left of a constraint, so these parts cannot change.

The constraint sources and why each keeps the invariant:

| Source | Why it keeps the invariant |
|---|---|
| a use of a name (rule 3), patterns, clause matching, `|>`, `! T = e` | the type of the used binding is on the left |
| annotations (value types) | a frozen variable is never put in; its fixed form is (A1, A2); a holder is shared only inside its own binding (A3) |
| `known.rs` (rule 13a and others) | a known type holds no solver variable of a finished group (debug assertion in `known_bound`) |
| `check_places`, `check_places_ty` | they only check |
| lowering, coverage, queries | they run after inference and only read |
| records (T3: `Without`, label maps) | the same direction rules; T3 must keep this assertion green |

**Debug assertion.** In `solve.rs`, when a lower bound is added to `v`, assert `frozen[v]` is not `Lower`; when an upper bound is added, assert it is not `Upper`. The assertion is on only in debug builds and only when `frozen` is not empty.

**Example.** In records.md 5.9, `user` is a call, so it is not generalized. Its type is `Codec(D, E)`. `D` is only at output places (the result of `decode`) and has the lower bound `{ age: Int, name: String }`: `Lower`. `E` is only at an input place (the parameter of `encode`) with the upper bound coming from the field encoders: `Upper`. So `Codec(User, _) = user` reads `User = { age: Int, name: String }`, and no later code can change it.

**A property that the tests check.** `x <: fixed(x)` always holds, because coalescing the bounds is how MLsub makes a type simpler. So `check_x: x = x` is accepted for every top-level `x` whose read type is closed.

### 3.4 Order and cycles

- **O1. Type needs of definitions.** In resolve, after `alias_circles` (:1175), compute for each `TypeId` the set `value_deps[t]` of top-level `DefId`s that the definition reads: by `Of` in its body, by an alias pattern's source, and, transitively, through the aliases it names (alias bodies and alias-pattern heads). Aliases have no cycles once the circular ones are removed, so a memoised DFS computes it in O(aliases + edges). `uses_value_type[t]` is `!value_deps[t].is_empty()` or "names a declaration or a value of another module". Most entries are empty.
- **O2. Type uses of a binding.** `type_uses(b)` is the set of top-level bindings `d` of the same module such that a type written anywhere in `b` has `Of(Def(d))` or names an alias `t` with `d ∈ value_deps[t]`. The walker is one generic visitor, `hir::for_each_type_in_binding`, that visits every `hir::Type` in a binding: typed patterns (bindings, parameters, clause patterns, type tests), `->`, `?`, `!` parts, env params and `Stmt::Env` (they hold no `Of` after E0531, but they may name aliases), and every inner binding statement. Both order.rs and the debug check of O6 use this one visitor, so no kind of place can be missed by one and seen by the other. The Lisp surface lowers to the same core tree and HIR, so it needs nothing more.
- **O3. Groups.** Groups are the SCCs of the **use edges only**, as today (order.rs :188-228). Then a second graph is made over the groups: an edge for each use between groups and for each type use between groups. Groups are typed in a topological order of that graph.
- **O4. E0530.** A type use whose two ends are in one group (a self loop included, through an alias), or a cycle of groups in the second graph, is **E0530**. The message names the chain: "the type of `num` is used in a type in `check` (through the alias `N`), but `num` and `check` are typed together: `num` uses `check`. A type may only use the type of a value that is already typed. Write the type, or move the use out of this group." The type uses in the cycle are dropped for ordering, and their reads are marked `Failed` (they give `Error` with no second message).
- **O5. Groups do not change.** A correct program has no E0530, so the groups are those of today and only their order is fixed. Needs, init order and E-needs-itself (`needs()`, order.rs :137) are computed from `uses()` only and do not change at all. A unit test compares the groups with and without type edges.
- **O6. No read before typing.** A read of a top-level value that is not typed yet is an **internal error**, never a fresh variable. In `conv`, an `Of` never reaches the `alias_depth > 64` fallback (infer.rs :858), and in `kconv` never the `Top` fallback (known.rs :108). A debug assertion checks that every annotation-reading `Of(Def(d))` of the same module finds `typed[d]`.
- **O7. Locals and other modules.** Locals need no edges: statements are typed in order, and V4 covers the cases that would read too early. Values of other modules are typed first (`module_order`, order.rs :255), so their read schemes always exist.
- **O8. Alias graph.** A binder gets edges to the head of its pattern and to every `TypeRef` inside the pattern, in the graph of `alias_circles`. So `Schema(A, B) = Codec(User, B)` with `Schema(User, _) = user` is ALIAS_CIRCLE (E-existing).
- **O9.** Records' L6 group check (T3, in order.rs) is not changed, because the groups are not changed.

### 3.5 Alias patterns

- **P1. Telling them apart.** On the core tree after macro step 1 (meta.md §4.4), an item `UpName [ "(" Type, … ")" ] "=" Type` is:
  - a **plain alias with a value type** `T = x`, when there is no head and the right side, with its `{ }` groups removed, is a value type. This is the definition reading of `x`.
  - an **alias pattern**, when there is a head and the right side, with its groups removed, is a value type (`x` or `a.x`).
  - otherwise a type definition whose head is a **list of type parameters**, with the existing rules of syntax.md §4.3: each part an `UpName` with no module and no arguments, and not the name of a type that may be written with no module name in the module.

  The class is decided after macro step 1, so a type macro on the right side that expands to `user` makes a pattern, the same way in both surfaces. In the Lisp surface `(= (Codec User _) user)` is a pattern and `(= (Pair A) (, A A))` is not, by the same rule.
- **P2. The head.** The `UpName` before the head must be written with no module name. An item that starts with `a.` is a binding (§2.2): `json.Codec(User, _) = user` is a binding with a constructor pattern. When such a binding gets an error and its value is a lone value name, the message gets the hint: "to take apart the type of `user`, import `Codec` and write `Codec(User, _) = user`". The head is looked up as a type and is not defined by the item. It is either:
  - a custom or built-in type with as many parameters as the head has parts (else TYPE_ARITY);
  - or an alias whose expansion, through aliases, is a single named custom or built-in type, which does not use a value type (else **E0532**). Its body is put in, with its parameters replaced by the parts of the head. Each parameter must be used at most once in that body and only at a place that is a pattern place (an argument of a named type or a part of a tuple); else E0532 with the hint "the alias `MyCodec` stands for `Codec((D) -> Int, D)`; write the custom type it stands for". The parts of the body with no parameter become **exact parts**: the part read there must be closed and equal to them (`τ <: T` and `T <: τ` on closed types), else E0535.
- **P3. Parts (TypePattern).**
  - A plain `UpName` (no module, no arguments) is a **binder**. It defines a type of the module with that name. A second definition in the module is E0208. The same binder twice in one pattern is E0208. A binder with the name of a type that may be written with no module name (imported or built-in) is **E0537**: "`Int` is a type; a name in an alias pattern defines a new type; use `_` to skip a part".
  - `_` is a **skip**: nothing is read or checked there, and that part may hold general variables or unfixed parts. It is `TypePat::Skip`, never `TypeKind::Hole`.
  - `m.T`, `T(P, …)` and `m.T(P, …)`, where `T` is a custom or built-in type, take that type apart. An alias there follows P2.
  - `(P1, …, Pn)` takes a tuple apart.
  - Anything else is **E0532**: a union, an intersection, a difference, a literal, a function type, a record type (T3 may add record patterns), a value type, a macro call.
- **P4. Matching.** Let τ be `read[x]` (the definition reading of the source), with no closedness required yet. Then, from the top:
  - at a named pattern `C(P1 … Pn)`: τ, made simpler and with aliases at its top expanded, must be exactly one member, a named type with the type constructor of `C` and n arguments (not a union, intersection or difference). Each `Pi` is matched with the i-th argument.
  - at a tuple pattern: one member, a tuple with the same number of parts.
  - at an exact part (P2): equal, as above.
  - at a binder: the part must be closed (E0533 or E0534). The binder becomes a **structural alias** of that closed type. It has no parameters and no constructors (`User(..)` in the exports is the existing error for aliases).
  - at a skip: nothing.
  - a general variable or an unfixed part where a named type or a tuple is needed: E0533 or E0534. Anything else: **E0535**: "the type of `n` is `5`, which does not match `List(E)` (§4.3)".
- **P5. When.** Each pattern, and each definition-reading site, is handled **once**, at the end of the group that types the last of its value deps. Sites whose deps are all in other modules, declarations or built-ins are handled at the start of their module, before its first group. At each of these points, items are handled in source order. The export check (part 3.8) runs at the same point. So the errors come at the definitions, in a fixed order, whether or not anyone reads the binder.
- **P6. Top level only.** Blocks have no type definitions, and an item that starts with `(` or `{` is a binding.
- **P7. Two meanings of `_`.** On the left of an alias pattern, `_` skips a part. Everywhere else in a type it is a hole that inference fills. A skip never reaches `show_type`, the interface, or `TypeKind::Hole`.

### 3.6 The cache and the cost

- `program.has_value_types` is true when the program has any `Of` or alias pattern. When it is false: order.rs does not walk types, `group` freezes nothing, no tables are filled. Edge counts and std check time do not change (a check in the tests).
- `program.read_targets: HashSet<DefId>`: every def named by an `Of` or a pattern source, in any module. Only these get `read[x]`.
- `Checker.def_reads: Vec<DefRead>`, indexed by `OfId` for definition sites, and the binders' entries in `Checker.bound: HashMap<TypeId, DefRead>`. `DefRead` is `Pending | Closed(Ty) | Failed`. P5 fills them. A use in `conv`, `kconv`, `expand` or `check_places_ty` clones a `Closed` entry. `Failed` gives `Error` with no new message. `Pending` gives `Error` when E0530 or E0531 was reported for it, and otherwise an internal error (and a debug panic).

### 3.7 Known types, places, literal types

- **K1. Known type** of a value type (`kconv`, known.rs :102):
  - top-level, imported, a declaration, a built-in, or a finished generalized local: `read[x]` (or the instance) with general variables, `Top` and unfixed parts as `_`;
  - a name still being worked out (A3): the type written at the top of its binding (`p: T`), when there is one, and `_` when not. So in `(p: User) { q: p = p ⏎ q.name }`, `q` has the known type `User`, and `q.name` works.
  - a definition reading: the known form of the `Closed` entry.

  It is never the known type of the name. After `x = "a" = s`, the name `x` has the known type `"a"`, but `y: x = t` lets any `String` in (the type of `x` is `String`), so the known type of `y` is `String`, and `y == "b"` is not E0520.
- **K2. Literal types.** The type is read with no widening: `port = 8080` then `Port = port` makes `Port` the type `8080`. To get `Int`, write the type at the source (`port: Int = 8080`). The same holds for records: `p = { name: "a" }` gives `{ name: "a" }`.
- **K3. Places (§4.4, E0518).** `check_places` walks HIR types and treats `Of` as opaque. A new `check_places_ty(&Ty, Pol)` checks the read type at the polarity of the place where `Of` is written, before general variables and `Top` are replaced: a general variable counts as a written type variable, and `Top` as `_` (which is allowed anywhere). `use = (f: pick) { f(1, 2, True) }`, with `pick: (A, B, Bool) -> A | B`, is E0518, because `(f: (A, B, Bool) -> A | B)` would be. A closed type always passes.
- **K4. Type tests.** `x: user` in a clause pattern is allowed. `Typed.type_tests` heads come from the read `Ty`, so lower.rs needs no change. A debug assertion checks that a `Head` is never a binder `TypeId`, because `head_cond`'s alias fallback would give `Tag(u32::MAX)`.

### 3.8 Exports, the interface, queries

- **X1. §2.4.** An exported binding whose top annotation holds `Of` counts as not written (`self.written`, infer.rs :85, :989), so its inferred type goes through `exported_type` (:569) and EXPORT_HIDDEN_TYPE. An exported binder, and an exported alias with `Of` in it (directly or through aliases), is checked at P5 on its read type with the same rule. The message names the source: "`User` is exported, but the type that `Codec(User, _) = user` gives it uses `Secret`, which is not exported". `hidden_types` (resolve.rs :2460) skips `Of` and these aliases.
- **X2. `Typed.alias_types: HashMap<TypeId, ShownType>`** holds the read body of each binder and of each alias with `Of` in it, with the alias parameters shown as their written names.
- **X3. package.rs.** `TypeShow::head` for such a type prints `Name(Params) = ` and `canonical_type(alias_types[t])`. So a change in a private source's type is a change to `type m.User`. That is correct and stated in §2.4. `TypeShow::ty` gets an `Of` arm that prints the written name, only for text that is not canonical (written types of host and external entries cannot hold `Of`, E0531). `value m.y` with `y: x` already compares the inferred scheme.
- **X4. query.rs.**
  - `show_type_prec`: `Of` prints `user` or `m.user`.
  - `Walk::ty`: `Of` pushes `Target::Local(v)` or `Target::Def(d)` at its span; the `m.` part already works through `target_at`. The head of a pattern pushes `Target::Type`; each binder is its own `TypeDef`.
  - `type_definition` of a binder: `User = <alias_types>` when types exist, else the written item `Codec(User, _) = user` (inference may be `None` under catch_unwind).
  - `symbols()`: the type symbols of an item come from `module.types` inside the item's span, so binders are listed.
  - `named_types`: an `Of` arm that expands through `alias_types` when present, and gives nothing when not.
- **X5. Shown type variables must be UpName before T2.** `display.rs` Namer and package.rs `TypeShow::param` (A2 phase 0, syntax.md notes) must show `A, B`; otherwise a shown `a` would read back as a value type. Checklist step 1 stops if this is missing.

### 3.9 Records, macros, Lisp

- **Records.** `{ x }` is `x`; `{ x: T }` is a record type. T2 resolves every `Of` to `ValueRef::Local` or `ValueRef::Def` in resolve, and collects, per binding, `type_value_names: Vec<(ValueRef, Span)>`, the hook for T3's resolve-time L5 (E0542) and L2a (E0543) checks. This answers records.md open question 10. Under A3, `{ k }` for a key parameter `k: K` would be the shared holder of `k`, so L2a is really needed; before T3 lands there are no key parameters. A read type with `Param` labels or a spread base variable is not closed, so it is E0533 in a definition. The 5.9 codec works as part 3.3 shows.
- **Macros.** `x(…)` and `a.m(…)` in a type are TypeCalls. `{ f }(…)` is not a type form. A value type in a template is a value name, with hygiene by marks (meta §5). `Definitions.definition(User)` returns the alias pattern item as written, so no inference result leaks into macro step 1. Typed macros (`Types`, meta §7.4) see binders with their read bodies, which exist because the source's group comes first; M4 must keep the type edges for code it expands.
- **Lisp rows for meta.md §2.3.** Value type: `x`, `a.x`, `{ x }` are written `x`, `(. a x)`, `{ x }` in type position; a lowercase symbol, or a lowercase last part, is a value type, and a list with a lowercase head is a macro call. Alias pattern: `(= (Codec User _) user)`, told apart by P1.

## 4. Errors

| Code | Name | When |
|---|---|---|
| E0530 | VALUE_TYPE_CIRCLE | a type use inside one group, or a cycle of groups through a type use (O4); message with the chain |
| E0531 | VALUE_TYPE_PLACE | a value type, or an alias that uses one, in a constructor field, a declaration, a host type or an environment type |
| E0532 | TYPE_PATTERN_FORM | a part that is not a type pattern; a head that is an alias not standing for one named type |
| E0533 | VALUE_TYPE_OPEN | a general variable or `Top` in a used part of a definition reading |
| E0534 | VALUE_TYPE_NOT_FIXED | a part in the unfixed set is read |
| E0535 | TYPE_PATTERN_MISMATCH | the type does not match the pattern, or an exact part differs |
| E0536 | VALUE_TYPE_OWN_SCOPE | V4: same pattern, same parameter list, own value |
| E0537 | TYPE_PATTERN_BINDER_IS_TYPE | a binder has the name of an imported or built-in type |

Codes used again: E0201, E0203, E0204, E0205, E0207, E0208, E0221, E0227, E0501, E0518, TYPE_ARITY, ALIAS_CIRCLE. A3's code for "a lowercase name in a type" is removed with its tests in the same change; those tests become value-type tests.

## 5. Why it is sound (summary)

- In the annotation reading, a value type is only another source of constraints, like `_` holes linked to each other. Holes and rule 3 are already sound.
- In the definition reading, the type is closed, so every use of an alias means the same type.
- Trusted places are excluded (E0531).
- A value is never read before it is typed (O6), and E0530 catches every cycle.
- Fixed types do not go stale (the invariant of 3.3, with its assertion).
- Principal types are kept: every value type is a function of a principal scheme found earlier, or of a variable shared inside the same binding (like a declared type variable). So the binding that uses it is typed by plain MLsub.

## 6. Records 5.9, step by step

```i
user = object |> field("name", string) |> field("age", int)
Codec(User, _) = user
show = (u: User) { u.name }
```

1. resolve: P1 makes the item a pattern; `Codec` is a custom type of arity 2; `User` is a binder; `_` is a skip. `value_deps[User] = { user }`. `type_uses(show) = { user }`.
2. order: `show` is typed after `user`'s group; groups are unchanged.
3. infer: at the end of `user`'s group, F3–F4 freeze `D` as `Lower({ age: Int, name: String })` and `E` as `Upper(…)`. P5 matches the pattern: `User` is `Closed({ age: Int, name: String })`.
4. `show`: `(u: User)` converts the binder through the cache.

## 7. Spec text

### 7.1 §2.4 Exports (add after the last bullet)

> - When a type uses the type of a value (§3), the type it stands for is checked after inference: an alias whose body uses one, directly or through other aliases, a type defined by an alias pattern (§4.3), and the type written at the top of an exported binding. The value need not be exported, but the types it gives must be. So the type that an alias pattern gives an exported type is part of what the module exports, and a change to the value it reads, also when that value is not exported, changes the exported type.

### 7.2 §2.5 Top-level bindings (add after the paragraph on the order of values)

> **Types that use values.** A type may use the type of a value (§3). Such a use does not run code, so it is not a need, and it does not change the order in which values are worked out. It changes only the order of inference: a binding whose types use the type of `x`, directly or through aliases (§4.3), is typed after the group of `x` (§4.7). If `x` is in the binding's own group, or is the binding itself, it is a compile error, and the message names the chain of uses.

### 7.3 §3 Type syntax

In the grammar, `Prim` gets `ValueType`:

```
Prim      = FnType | TupleType | NamedType | TypeCall | ValueType | Literal | "_" | "{" Type "}" .
ValueType = [ name "." ] name .
```

The bullet "A lowercase `name` in a type, or `a.x` … is kept for a later use, the type of a value, and is a compile error. …" becomes:

> - A lowercase `name` in a type, with no `(` after it, is the **type of a value**: the type of the value that `name` names at the place of the type. It is found as the name in an expression at that place is found (§5). `a.x`, with a lowercase `x` and no `(` after it, is the type of the member `x` of the module `a` (§2.3), which must be exported. It is never a field of a value: when a local name covers the module `a`, `a.x` in a type is a compile error, so `(p: User, q: p.name) { q }` is a compile error. `{ x }` is the same as `x`. §4.7 says which type it is.
> - A type of a value may not name a name bound by the same pattern, a parameter of the same function in that function's own parameter types or its `->`, `?` and `!` parts, or a name inside its own value. So `(x, y: x)` and `(x) -> x { x }` are compile errors, also when there is an outer `x`.
> - The type of a value is a way to write a type that inference finds. It is not a new kind of type, and it never makes a value general: `(f: id)` gives `f` one type in the whole body, as `(f: (A) -> A)` with a new `A` would.
> - A type of a value may not be used in a field of a custom type (§4.3), in the type of a declaration (§10.1), in an environment type (§9.2), or in a host type (§9.7). The same holds for an alias that uses one, directly or through other aliases. These types must be known before types are inferred.

The bullet on `(` after a lowercase name (macro calls) keeps its text.

### 7.4 §4.3 Custom types (full text)

```
TypeDef     = UpName [ TypeHead ] TypeBody .
TypeHead    = "(" Type { "," Type } [ "," ] ")" .
TypeBody    = "{" { Constructor } "}" | "=" Type .
Constructor = UpName [ "(" Field { "," Field } [ "," ] ")" ] .
Field       = [ name ":" ] Type .
TypePattern = UpName | "_"
            | [ name "." ] UpName [ "(" TypePattern { "," TypePattern } [ "," ] ")" ]
            | "(" [ TypePattern { "," TypePattern } [ "," ] ] ")" .
```

> - `T { C1 C2 … }` is a custom type. Every constructor has zero or more fields. A field may have a label.
> - `T {}` is a type with no constructors. i code may not make its values. They only come from the host (§10).
> - `T = Type` is a type alias. `T` is the same as the type on the right. An alias may not use itself, directly or through other aliases or alias patterns.
> - **Which form.** An item `T( … ) = V`, whose right side is only the type of a value (§3), also inside `{ }`, is an **alias pattern**. Every other head is a **list of type parameters**. Which reading an item has depends only on how its right side is written, never on the names in scope, so an import may make an item an error but never changes what it means.
> - In a list of type parameters, every part must be an `UpName` with no module name and no arguments, and it may not be the name of a type that may be written with no module name in the module (a type of the module, an imported `UpName`, or a built-in type). When a part is not such an `UpName`, as in `Box(a)` or `Box(List(A))`, the message says that a type parameter is an `UpName`. When it is the name of a type, as in `Pair(Int) = (Int, Int)`, the message says that a type parameter may not have the name of a type.
> - The type parameters are the type variables of the definition: `Option(A) { Some(A) None }`. Every type parameter must be used on the right, and every type variable on the right must be a parameter. The type parameters of one definition must have different names.
> - A type of a value (§3) in an alias stands for one type, with no type variable and no `_` in it (§4.7). So `Same = id`, with `id = (x) { x }`, is a compile error. A type of a value may not be used in a field of a custom type, even through an alias.
> - **Alias pattern.** In `T(P1, …, Pn) = v`, `T` is a type that already exists, written with no module name; it is not defined by the item. (`a.T(…) = v` is a binding, §2.2; to take apart a type of another module, import its name.) `T` is a custom type or a built-in type with n parameters, or an alias that stands for one such named type. The item takes apart the type of the value `v` (§4.7). Each `Pi` is a type pattern:
>   - An `UpName` with no module name and no arguments always defines a new alias with that name, the same as a lowercase name in a value pattern always binds. It is a type of the module, defined one time only, and it may not have the name of an imported or built-in type. It stands for the part of the type at its place, which must have no type variable and no `_` in it.
>   - `_` takes any part and defines nothing. It is not a hole.
>   - A named type with a module name or with arguments takes apart a type with exactly one member, of that type constructor, and matches its parts with the type arguments.
>   - A tuple pattern takes apart a tuple type with the same number of parts.
>
>   When the type does not have the form of the pattern, it is a compile error. Any other form in a pattern, such as a union, a function type or a literal type, is a compile error.
>
>   ```i
>   Codec(D, E) { Codec(decode: (String) -> D, encode: (E) -> String) }
>   make(D, E) = (d: (String) -> D, e: (E) -> String) { Codec(decode: d, encode: e) }
>   size = make((s) { string.length(s) }, (n: Int) { int.to_string(n) })
>   Codec(Size, _) = size     // Size is Int
>   ```
> - A name defined by an alias pattern is an alias: it has no parameters and no constructors, and it is exported as `T`.
> - Custom types are **nominal**: two different definitions make two different types, even if they have the same form.

### 7.5 §4.7 Type inference (add after the bullet on groups)

> - **Types of values.** A type of a value `x` (§3) is read from the type that inference finds for `x`.
>   - **In an annotation.** In a type written in a binding (on its pattern, in parameters, `->` parts, clause patterns, and inner bindings), `x` has the type that the name `x` has as an expression at that place (§4.8 rule 3). Each general type variable of `x` becomes a new type variable, one for each variable, at each place where `x` is written, as `_` does. For example, with `id = (x) { x }`, `twice = (f: id, n: Int) { f(f(n)) }` gives `f` the type `(A) -> A` for a new `A`.
>   - **A name being worked out.** A parameter, a name of a clause pattern, or a local binding that is not generalized has one type, and the annotation shares it. So in `(xs: List(Int)) { acc: xs = [] … }`, `acc` and `xs` have one type: the value of `acc` must fit in the type of `xs`, and what code asks of `acc` it asks of `xs`.
>   - **In a type definition.** In an alias or an alias pattern (§4.3), the type must have no type variable and no `_` in the parts that are used. `Same = id` is a compile error; `Codec(Size, _) = size` is correct.
>   - **The fixed type.** A top-level binding that is not generalized is read through its **fixed type**: its type when its group is typed, made simpler with the other bindings of its group. A type variable that is only at output places becomes the union of the types that come to it (`Never` when there are none). A type variable that is only at input places becomes the types it goes to. Example: with `user = object |> field("name", string)`, the decode side of the type of `user` is `{ name: String }`.
>   - **Not fixed.** A type variable at both kinds of places is not fixed, and a type that uses such a part is a compile error, because later code could still change it. Write the type of the binding. Example: `g = id(id)` is not generalized, and its type `(A) -> A` has `A` at both kinds of places, so `(h: g)` is a compile error.
>   - **Why fixed types do not change.** The uses of a binding only give types that go out of it: in a use, the binding's type is the type of the value that is given. So the types that came to an output place, and the types that an input place goes to, cannot change after its group. This is a rule of the language: later features must keep it.
>   - The type that is read must keep the rules of places of §4.4 at the place where it is put, as a written type must.
> - **Order.** A type of a value is read when the value is typed. A binding whose types use the type of a top-level binding of its module, directly or through an alias, is typed after the group of that binding. A type never puts two bindings in one group: when the types of a binding use the type of a binding of its own group, or its own type, it is a compile error.
> - The value restriction is the same for types of values: a binding that is not generalized gives no general type variable, and a type of a value never makes a value general.

### 7.6 §4.8 Typing rules (edits)

- In the pattern types, "`x: T`: `T`, with a new type variable in place of each `_` in it" becomes "`x: T`: `T`, with a new type variable in place of each `_` in it, and each type of a value in it read as §4.7 says".
- In **Names in patterns**, after "The name `x` of `x: T` gets `T`", add:

> A type of a value `y` in `T` gives, as a known type, the type that §4.7 reads for `y`, with `_` for each type variable in it. For a parameter, a name of a clause pattern, or a local binding that is not generalized, it is the type written at the top of its binding, or `_` when there is none: in `(p: User) { q: p = p … }`, `q` has the known type `User`. It is never the known type of the name `y`, which may have fewer values: after `y = "a" = s`, `z: y = t` gives `z` the known type `String`, not `"a"`.

## 8. Representation and files

### 8.1 HIR (`crates/ilang-check/src/hir.rs`, around :189 and :273)

- `TypeKind::Of { value: ValueRef, site: OfId, part: Option<OfPart> }`. `ValueRef` is `Local(VarId)` or `Def(DefId)` (never `Ctor`: the name is lowercase). `OfPart { pattern: AliasPatId, binder: u32 }` is set only in the body of a binder.
- `Program.of_sites: Vec<OfSite { module, span, value: ValueRef, reading: Reading }>`, with `Reading::Annotation | Reading::Definition(TypeId)`.
- `Program.alias_patterns: Vec<AliasPattern { module, span, head: TypeRef, head_span, pattern: Vec<TypePat>, value: ValueRef, value_span, binders: Vec<TypeId> }>`.
- `TypePat = Bind(TypeId, Span) | Skip(Span) | Named { ty: TypeRef, args: Vec<TypePat>, span } | Tuple(Vec<TypePat>, Span) | Exact(Type) | Error`. `Exact` only comes from an alias head (P2).
- Each binder is an ordinary `TypeDef { name, span: binder span, params: [], body: TypeBody::Alias(Type { kind: Of { value, site, part: Some(..) } }) }`. No new `TypeBody` variant.
- `Program.value_deps: Vec<Box<[DefId]>>` and `uses_value_type: Vec<bool>`, indexed by `TypeId`; `read_targets: HashSet<DefId>`; `has_value_types: bool`; `type_value_names` per binding (the T3 hook).
- `hir::for_each_type_in_binding` (O2).

### 8.2 Files and regions

- **ilang-syntax** (after A3): `ast::TypeKind::Value` exists. The A3 error for a lowercase name in a type is removed. `type_vars()` never collects `Value`. The P1 classification is made on the core tree; the Lisp printer and reader get the rows of 3.9. The `json.Codec(…) = user` hint is added where the binding's error is reported (resolve).
- **resolve.rs:**
  - `collect_type` (:528): P1. For a pattern, register each binder as a `TypeDef`, with E0208 and E0537; do not register the head.
  - `type_defs` (:1098): resolve the head (P2) and the parts (P3, E0532), and the value name to a `ValueRef::Def` (DefIds exist in phase 3; scope values and `defs_by_name`).
  - `ty` (:1329): a `Value` arm. In annotations, `find` (:1834) with V4 (a `pending_binders` stack checked first, E0536); in definitions, the phase-3 lookup. `a.x` through `member` (:1853), E0207 when a local covers `a`.
  - `function` (:2133) and block statements: push the names being bound onto `pending_binders` while their types are resolved; mark the own value of a binding.
  - `alias_circles` (:1175): binder edges to the head and the parts (O8). After it: `value_deps`, `uses_value_type` (memoised DFS), `read_targets`, `has_value_types`.
  - E0531 checks, using `uses_value_type`, in constructor fields, `external_type`, `env_type` (:1445), `host_part` (:1568), `host_fn`, `host_type`. A hit is E0531 only, not ENV_TYPE or EXTERNAL_HOST_TYPE.
  - Exhaustive arms: `named_types`, `for_each_child`, `subst` (`Of` stays), `hidden_types` (:2460, skip), `expand_alias` (:1412, never reaches env and host paths after E0531), `type_vars` (never collect).
  - `type_value_names` per binding.
- **order.rs:** `type_uses` with the shared visitor; the group graph of O3 next to `tarjan_scc` (:220); E0530 with a BFS chain; `needs()` (:137) and init (:230) unchanged; the unit test of O5.
- **types/infer.rs:**
  - `Checker`: `first_var`, `frozen: HashMap<V, Fixed>`, `unfixed: HashSet<V>`, `read: HashMap<DefId, Ty>`, `def_reads`, `bound`.
  - `group` (:233): F1–F5 at its end, then P5 for the sites and patterns it completes; the module-start pass of P5.
  - `read_value_type(site, level)`: A1–A3 and the definition reading.
  - `conv` (:852): an `Of` arm; no `alias_depth` fallback for `Of` (O6).
  - `expand` (:953) and `check_places` (:603): `Of` is opaque; `check_places_ty` (K3).
  - `walk_variance` and `variances`: `Of` adds nothing (it never appears in a field, E0531).
  - P4 matching (a new `alias_pattern.rs` next to `infer.rs`), E0533–E0535.
  - `written` (:989) and `exported_type` (:569): X1. `Typed.alias_types`.
- **types/solve.rs:** the debug assertion of 3.3.
- **types/known.rs:** `kconv` (:102) arm (K1); the assertion in `known_bound`.
- **types/mod.rs:** `Typed.alias_types`.
- **query.rs:** X4.
- **crates/ilang-cli/src/commands/package.rs:** X3.
- **crates/ilang-check/tests/hir.rs** (the printer): `Of` and the patterns.
- **lib.rs:** codes E0530–E0537.
- **docs:** spec §2.4, §2.5, §3, §4.3, §4.7, §4.8 (part 7); syntax.md summary bullets become "done in T2"; meta.md §2.3 rows and §11 question 2 answered; records.md open question 10 answered and line 715 changed to the code split of decision 14.

## 9. Ordered checklist for T2

0. Confirm the code split with the coordinator and T1 (T1 E0523–E0529, T2 E0530–E0537, T3 E0540–E0549). Change records.md line 715 to say so.
1. Rebase on A1, A3 and T1. Confirm `ast::TypeKind::Value`, the head of `ast::TypeDef` read as types, and UpName shown type variables (display.rs Namer, package.rs `TypeShow::param`, query.rs `builtin_head`). **Stop and ask if any of these is missing.**
2. Add the codes E0530–E0537 to lib.rs `code`.
3. Add the HIR types (`TypeKind::Of`, `OfSite`, `AliasPattern`, `TypePat`, the tables, the visitor). Add arms to every exhaustive match: resolve, infer, known, query, package, tests/hir.rs. Each new arm reports an internal error until its step below; there is no silent fallback.
4. resolve.rs: P1–P3 and binders, the `Value` arm, `a.x`, V4 and E0536, alias graph edges, `value_deps`, `uses_value_type`, `read_targets`, E0531, the `json.Codec` hint, the T3 hook. Remove A3's lowercase error and its tests.
5. order.rs: type uses with the shared visitor, the group graph, E0530 with the chain. Unit test: groups, needs and init are unchanged with and without type edges.
6. infer.rs, solve.rs, known.rs: fixed forms (F1–F5) and the solver assertion; A1–A3 and the definition reading; `conv` and `kconv`; `check_places_ty`; P4–P5; the caches; the exported checks.
7. `Typed.alias_types`; query.rs; package.rs.
8. Spec text (part 7) and the doc edits of 8.2.
9. Tests (part 10). Run `check-all.sh` and the std checker. Confirm no change in goldens without value types, and that std check time is unchanged within noise (S2's script).

## 10. Tests

**tests/spec:**

- `3/value_type.i` (run): `twice = (f: id, n: Int) { f(f(n)) }`; `(xs: List(Int)) { acc: xs = [] … }`; `{ x }` is `x`; `m.x` from an imported module.
- `3/value_type_errors.i`: `x(Int)` in a type is still the macro error; a local `a` covering module `a` in `a.x` is E0207 with the field message; `(p: User, q: p.name)` is E0207; `m.private` is E0204.
- `3/value_type_own_scope.i`: E0536 for `(x, y: x)` with a top-level `x` (the message names it), `(x) -> x { x }`, a block `x: { x } = 1` with an outer `x`, a clause pattern `{ (a, b: a) -> … }`, and a local `f = (g: f) { g }`.
- `3/value_type_places.i`: E0531 for a constructor field, an alias used in a constructor field, `@external` with `(user) -> Int`, `! T = e`, `x ! T`, a host service type.
- `4.3/alias_pattern.i` (run): the Codec example (`Size` is `Int`, `(n: Size)` accepts `1`); `List(E) = xs`; `Pair((A, _), _) = p`; `Codec(_, _) = size`; `T = size_fn` (plain); a head alias `MyCodec(D) = Codec(D, Int)` with `MyCodec(S) = size`.
- `4.3/alias_pattern_errors.i`: E0532 for `Codec(Int | String, _) = size`, for a head alias `F(A) = Box((A) -> Int)`, for a function-type part; E0535 for `List(E) = n` with `n = 5`, and for an exact part that differs; E0533 for `Same = id` and `Codec(D2, _) = poly`; E0537 for `Codec(Int, _) = size`; E0208 for `Pair(A, A) = p` and for a binder that is also a custom type of the module; ALIAS_CIRCLE for `Schema(A, B) = Codec(User, B)` with `Schema(User, _) = user`.
- `4.3/alias_pattern_qualified_head.i`: `json.Codec(U, _) = u` parses as a binding, and its error has the import hint.
- `4.3/alias_head_import/` (project of two modules): `Pair(A) = (A, A)` is correct; after importing a type `A`, it is the type-parameter error, not a pattern. `Codec(User, _) = user` is correct; after importing a type `User`, it is E0537, not a test against `User`.
- `4.3/alias_pattern_unread.i`: a pattern that nobody reads still gives its E0535, at the item, in source order.
- `4.7/value_type_order.i`: a reader written before its source is typed after it; the init order is unchanged (prints show the source order); no E-needs-itself from type uses alone.
- `4.7/value_type_circle.i`, E0530: `x: T = 1` with `T = x`; `a = (n: b) { n }` with `b = a(1)`; `user: Codec(User, _) = …` with `Codec(User, _) = user`; a chain through two aliases (the message names it).
- `4.7/value_type_fixed.i`: `size` (not generalized) is read as `Codec(Int, Int)`; `copy: size = make((s) { "a" }, …)` is E0501, so `size`'s type is not widened; `check: size = size` is accepted; `g = id(id)` used in `(h: g)` is E0534; an encode side read with `Codec(_, Enc) = user` is E0533 or E0534 with "write the type of `user`".
- `4.7/value_type_fixed_shared.i`: variables shared across groups and across modules: `a = ref_like()` in module m, `b = wrap(m.a)` in module n, `B = b`; and a group whose variable has bounds from an earlier group at both kinds of places. Each checks that a variable at both kinds of places is never missed (E0534) and that a one-sided one gives the right fixed form.
- `4.7/value_type_places.i`: `use = (f: pick) { f(1, 2, True) }` is E0518.
- `4.8/value_type_known.i` (run): `(p: User) { q: p = p ⏎ q.name }`; after `x = "a" = s`, `y: x = t` then `y == "b"` gives no E0520; `port = 8080 ⏎ Port = port ⏎ bad: Port = 8081` is E0501.
- `2.4/export_value_type/`: an exported binder bound to a private custom type is E0227 with the source in the message; an exported `y: secret_val` whose type uses a private type is E0227; a private source with only exported types is fine.

**Checker goldens** (`crates/ilang-check/tests/types/value_types.i`, `alias_patterns.i`): the shown schemes of readers, the read types with UpName variables.

**Unit tests:** order.rs, groups equal with and without type edges and needs/init unchanged; the F3 polarity walk on hand-built solvers (cycles, invariant arguments, earlier-group variables); `alias_pattern.rs` matching; the solver assertion fires on a forced wrong-direction bound.

**Query tests:** hover on `size` in a type shows its scheme; hover on a binder `User` shows `User = { … }`, and the written pattern when inference fails; go to definition from `User` goes to the binder, and from `user` in a type to the binding; the outline lists the binders.

**Package test** (`crates/ilang-cli/tests/package_commands.rs`): changing a private `size` to give `String` reports `type m.Size` as changed; an exported alias `T = List(x)` is compared by its read body; nothing changes when nothing changed.

**Property test:** for every top-level binding of std whose read type is closed, a generated `check_x: x = x` is accepted. With the debug solver assertion on, the whole std and the spec tests run with no assertion failure.

**Performance:** std check time unchanged within noise; edge counts equal for programs with no `Of`.

## 11. Risks

- **E0531 on constructor fields** limits nominal wrappers such as `Account { Account(User) }` with `User` from a pattern. Users must use aliases (or records after T3). Lifting it needs a sound order for field access by label.
- **The fixed-type invariant** is a rule every later feature must keep (mutable cells in i, new annotation forms, known-type code that adds constraints, records). The solver assertion catches breaks in tests; it is off in release builds.
- **The polarity walk (F3–F4) is new code with soundness weight.** A mistake through bounds between variables of different groups or modules could give a fixed type that is too small (false E0501) or miss a variable at both kinds of places. The tests of `value_type_fixed_shared.i` and the unit tests target this.
- **check_places_ty** checks solver types, not written ones. Read types simplified differently from written types (the `has_both` fallback in `generalize`) may be rejected or accepted in surprising ways.
- **The head needs an `UpName` with no module name.** `json.Codec(User, _) = user` is a binding. The hint helps, but a correct binding of that form gets no hint.
- **Literal types are not widened.** `Port = port` is `8080`, and through exported aliases a private value's body change can change the interface. Intended, but a new source of interface churn.
- **Head aliases** (P2) add exact parts and an equality test on closed types. It is a small new piece of the matcher.
- **Codes.** If T1 needs more than seven codes, or the split is not confirmed, the codes clash on merge.
- **Order with T3.** T3's resolve-time checks must walk `Of` (the hook). The plan keeps T2 before T3, and there are no key parameters before T3, so there is no gap.
- **A3 and A2 prerequisites.** Without UpName shown variables, hover and interface text read as value types (step 1 stops).
- **Never as a fixed form.** A part with nothing coming to it is `Never` (part 3.3). `Codec(X, _) = c` then gives `X = Never`, which is sound but may surprise.

## 12. Open questions

1. Should value types be allowed in constructor fields later, with a "prelude group" rule that types the sources before the custom type is used?
2. Should a part with nothing coming to it (`Lower(Never)`) be E0533 instead of `Never`?
3. Left-to-right scoping of parameters in types (`(a, b: a) -> a`) as a later extension. It cannot change meaning, because today it is E0536.
4. Function-type patterns and record patterns (T3) in alias patterns.
5. Should matching relax "exactly one member" for a union with `Nil`, as known-type matching does?
6. A lint for a correct binding `a.C(…) = x` that looks like an alias pattern.
