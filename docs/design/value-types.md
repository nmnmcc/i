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
4. **A top-level binding that is not generalized is read through its fixed type, and the read fixes it.** At the end of its group, each type variable in a part that some type reads becomes the union of the types that came to it, which is the type §4.7 already shows for it. The solver then holds that variable to that type for good (it is *pinned*, part 3.3), as if the type had been written at the binding. A variable that nothing has come to yet, and a group whose typing touched a variable of an earlier group that is not pinned, are *not fixed*, and reading them is an error. *Reason:* a rule based on where a variable sits (input or output places) does not hold in this solver: service arguments in an environment are compared both ways (§4.5 rule 4), and `add_bound` compares bounds on the same side of related variables, so a variable "at output places only" can still gain lower bounds later. Pinning does not depend on any such rule: after the pin, every later constraint on the variable is a check against a closed type. Without a fixed type, `y: user = c` would add lower bounds to the decode variable of `user`; one annotation would change another binding's type, and `Codec(User, _) = user` would depend on the order of the groups. The cost of pinning is stated plainly: reading the type of `x` can make later code that would widen `x`'s type an error, exactly as writing the type at `x` would.
5. **A type use of a value is a new kind of edge, in the group graph only.** It is never a need and never changes the init order (§2.5). A cycle with a type edge in it is an error, E0530. *Reason:* a type reads no value at run time. Since every such cycle is an error, the groups of a correct program are the same as today (part 3.4).
6. **An item is an alias pattern exactly when its right side is a value type.** The right side alone decides it, never the names in scope. *Reason:* an import can then turn a correct program into an error, but never give it a new meaning. Nothing is lost: a generic alias whose whole body is one value type is always an error (its parameters are unused, E0221, and a value type in a definition must be closed).
7. **Inside a pattern, a plain `UpName` always defines a new type, `_` skips its part.** *Reason:* the same as a lowercase name in a value pattern, which always binds. If `User` could mean an existing type when one is in scope, an import would change the meaning.
8. **The head of a pattern may be an alias that stands for one named type.** *Reason:* a library may rename `Codec`; a user should still be able to take its values apart. The alias body is put in, and its parts become parts of the pattern (part 3.5).
9. **Value types are not allowed in constructor fields, external declarations, host types and environment types** (E0531), also through an alias. *Reason:* these are trusted, or converted before any group (`infer.rs` around :157-215, `variances`), or checked eagerly in resolve (`env_type` :1445, `host_part` :1568), or read by field access `v.f` with no syntactic use to order by. Custom types stay complete before inference, as meta.md §4.4 promises.
10. **A pattern is matched, and a definition with a value type is read, once, at the end of the group of its source.** *Reason:* the order of errors and `Typed.alias_types` must not depend on who reads a binder first, and a pattern nobody reads must still be checked.
11. **`a.x` in a type is always a member of a module, never a field.** When `a` names a value at that place, it is an error (E0207, "a type may name a value, not a field of a value; write the field's type"). This holds for a local that covers the module `a`, and also for a name that is not yet in scope there but is being bound by the same pattern or parameter list (V4): `(p: User, q: p.name)` is E0207 whether or not a module `p` is imported. *Reason:* it never silently changes meaning, and a field type would need the known type, which decision 2 rules out.
12. **`x(` and `a.m(` in a type stay macro calls** (meta.md). `f(A)` on a value name is never type application. *Reason:* one meaning per form.
13. **Each binder of a pattern is an ordinary `TypeDef` whose body is an alias of a new `hir::TypeKind::Of`.** There is no new `TypeBody` variant. *Reason:* exports, E0208, query targets and meta `Definitions` keep working unchanged.
14. **T2 takes the codes E0530–E0537.** T1 keeps E0523–E0529, T3 keeps E0540–E0549. *Reason:* the streams merge in different orders and must not collide. records.md line 715 ("T1 and T2 number their new codes from E0523 upward") is updated when T2 lands (checklist step 0).
15. **No cost for programs without value types.** No new edges, no new passes, no new solver variables, no allocation beyond empty vectors. *Reason:* std and most programs never use the feature.

## 3. Exact rules

### 3.1 Form and lookup

- **V1.** In a written type, a lowercase `name` with no `(` after it, or `a.x` with a lowercase `x` and no `(` after it, is a **value type**. `{ x }` and `{ a.x }` are the same as `x` and `a.x`. `{ x: T }` is a record type (T3). With `(` after it, it is a `TypeCall` (macros).
- **V2.** `x` is looked up as a value name at the place of the type: locals, then top-level names of the module, then imported names, then built-in values (§2.3, §5). A name that is a module is E0205. An unknown name is E0201.
- **V3.** `a.x` is the member `x` of the module `a` (`member()`, resolve.rs :1853). It must be exported (E0204, E0203). Before `a` is looked up as a module, it is looked up as a value name with the V4 stack: when `a` is a local in scope, or a name on the `pending_binders` stack (a name of the same pattern or parameter list, or the own value), it is E0207 with the message of decision 11. So `(p: User, q: p.name)` and `(p: User) -> p.name { … }` are E0207 also when a module `p` is imported, as syntax.md §3 says.
- **V4. Own scope.** A value type may not name:
  - a name bound by the same pattern (a binding pattern, a parameter pattern, a clause pattern): `(x, y: x)`, `x: { x } = …`, `{ (a, b: a) -> … }`;
  - a parameter of the same function, in that function's parameter types or its `->`, `?` and `!` parts: `(x) -> x { … }`;
  - a name inside its own value: a local `f = (g: f) { g }`, or a top-level `f = (g: f) { g }`.

  Each is **E0536**. The same stack is checked for the `a` of `a.x`, which gives E0207 (V3), not E0536. The message names the outer binding that the name would otherwise have found, if there is one: "`x` is bound by this parameter list, so a type here may not name it; without this rule it would name the top-level `x` at line 3". *Reason:* at c69515c, `function()` (resolve.rs :2133) resolves parameter patterns, with their types, before it pushes the parameter frame, and resolves the `->` part before it too; a block binding's type is resolved before its name is bound. So without V4 these names would silently find an outer `x`. Left-to-right scoping (`(a, b: a) -> a`) is a later extension that cannot change meaning, because today it is an error (open question 3).
- **V5.** A value type that names a binding with label parameters is E0542 (records L5), and one that names a key parameter is E0543 (records L2a). T3 checks both in resolve; T2 gives it the names (part 3.9).

### 3.2 The two readings

Every value type in HIR is `Of { value, site, part }` (part 8). Its **site** says which reading it has.

- **Annotation reading.** Any type written in a binding: the type of its pattern, parameter types, `->`, `?` and `!` parts, clause patterns and type tests, and the same places in inner bindings. (Environment types, `Stmt::Env` and env params may not hold a value type, decision 9.) Here `Of(x)` is:
  - **A1.** `x` is top-level (this module or another), a declaration, or a built-in: the **read scheme** `read[x]` (part 3.3). Each general variable becomes a new variable at the current level, one per variable for this place (all occurrences of one general variable at this place share it). An annotation uses the whole type, so every variable that is not general was pinned when `x`'s group ended (F4), or is **unfixed** and gives **E0534**.
  - **A2.** `x` is a generalized local whose binding is finished: `instantiate(scheme)`. A variable of a finished top-level group found in the result is pinned (the solver reads it as its fixed type) or gives E0534. A local is read at a place inside the same top-level binding, so this needs no new pin: the variables of an earlier group that a local's scheme holds were pinned or not at that group's end.
  - **A3.** `x` is a parameter, a clause name, or a local that is not generalized, all inside the top-level binding being typed: its holder type, **shared**. So `(xs) { acc: xs = [] … }` gives `acc` and `xs` one type. The annotation puts that type on the right of a constraint (`List(Never) <: τxs`), so the value of `acc` must fit in the type of `xs`, and every use of `acc` asks the same of `xs`. This is sound: the holder is a variable of the binding being typed, and it is linked to other variables only by constraints that the binding's own code made. A pinned variable of an earlier group is a closed type to the solver (F5), so no constraint through the holder can change it. A key parameter is never read this way: it is E0543 (V5).
- **Definition reading.** The body of an alias, or the source of an alias pattern. `Of(x)` is `read[x]`, and every part that is used must be **closed**: no general variable, no `Top`, no unfixed variable. Otherwise **E0533** (a general variable or `Top`) or **E0534** (unfixed). The message says which part is open, shows it with the written name of the alias, never with `_`, and says "write the type of `x`". For an alias pattern, only the parts below a binder are used (part 3.5). The result is closed, so it is stored once and each use is a clone (part 3.6).

### 3.3 Fixed types (solver terms)

An earlier draft decided which variables are fixed by where they sit (only at output places, only at input places). That does not hold in this solver: service arguments in an environment are compared for equality both ways (`constrain_in`, the `env` arm, §4.5 rule 4), and `add_bound` (solve.rs :385) calls `compare` on the bounds on the same side of the variables it reaches, which makes invariant arguments equal and joins covariant ones. So a variable that sits only at output places can still gain lower bounds from later code. The rules below do not depend on places. A variable that a type reads is **pinned**: the solver reads it as a closed type from then on.

- **F1. When.** At the end of `Checker::group` (infer.rs :233), after the holders are linked and after `generalize` (:2108), for a group that has a **read target** (`program.read_targets`, part 3.6). Other groups do nothing.
- **F2. Read parts.** `program.read_parts[x]` says which parts of `x`'s type some site uses: `Whole` when an annotation site or a plain alias `T = x` names `x` (anywhere in the program), and otherwise the paths (argument positions from the top) of the binders of the alias patterns whose source is `x`. Resolve knows every site of every module, so a site in a later module counts. A part under `_` only is never read.
- **F3. Open groups.** At the start of `group`, record `first_var`, the number of solver variables. While the group is typed, the solver sets `group_open` when a constraint meets a variable that is not general, has an index below `first_var`, and is not pinned: as an operand of `constrain_in`, or as a variable bound reached by `add_bound` and `reach`. When `group_open` is set, every variable that is not general in a read part of a read target of the group is **unfixed**. *Reason:* such an earlier variable may have gained bounds from a group that is not an ancestor of this one, so what came to this group's variables would depend on the order of the groups. The E0534 message says "the type of `x` shares a part with an earlier binding whose type is not fixed; write the type of `x`, or of the binding it uses".
- **F4. Fixed forms and pins.** For each variable `v` that is not general and is in a read part (it is a variable of the group, since earlier ones are pinned or make the group open), `fixed(v)` is the union of the fixed forms of its lower bounds:
  - a bound with a type constructor or a function type, with each variable inside replaced by its own fixed form;
  - a variable bound `u`: `fixed(u)` when `u` has lower bounds, and nothing when it has none.

  `v` is **unfixed** when the union is empty (nothing has come to `v` yet), when a variable inside one of its bounds is unfixed, or when the form would hold `v` itself (a recursive bound; the message notes "a type that has itself inside it"). Otherwise `fixed(v)` is closed, and `v`, with every variable whose form was used to make it, is **pinned**: `pinned[v] = (fixed(v), site)`, where `site` is the first site that reads `v` in source order. `fixed(v) <: v` holds already (it is the union of `v`'s lower bounds), and `v <: fixed(v)` holds for every value that has come; a debug assertion checks that the constraint `v <: fixed(v)` gives no failure. Unfixed variables are not pinned and are listed in `unfixed: HashSet<V>`.
- **F5. A pinned variable is a closed type.** `Solver.pinned: HashMap<V, Ty>` (empty when there are no value types). `constrain_in` replaces a pinned variable operand by its type before it matches. `reach` and `compare` read a pinned variable met as a bound as its type, a bound with a type constructor. `extrude` and `freshen` never meet one, because it is replaced first. So every later constraint on `v` is a check of a closed type against `fixed(v)`, and nothing that `add_bound` derives can flow through `v`'s old bound lists. A later failure against a pinned type gets the note "the type of `x` was fixed at line N, where a type reads it". The solver `Ty` gets no new variant.
- **F6. The read scheme.** For each read target `x` of the group: `read[x]` is its scheme with every pinned variable replaced by its type. General variables stay as general variables. Unfixed variables stay as variables and are found through `unfixed` when read. Parts outside `read_parts[x]` are not looked at.
- **F7. Showing.** The fixed forms may be made simpler with `display::simplify` for showing only. What is fixed is decided by F3–F4 on raw bounds, never by what simplify removes.

**The read type is the shown type.** §4.7 shows a variable that is not generalized as the union of the types that come to it. After the pin, nothing more can come to it, so the type shown for `x` (hover, `value m.x` in the package interface, messages) and the type that `x` stands for in a type are the same. A variable that nothing has come to is shown as `Never`, and reading it is E0534; it is never read as `Never`, and never as the types it goes to.

**Why fixed types do not change.** Not because of a rule that every feature must keep, but because the solver reads a pinned variable as its type (F5). The one rule for later code: a bound is added to a variable only through `constrain`, `add_bound` or `compare`, never pushed onto its lists directly. A debug assertion in `add_bound` checks that the variable is not pinned.

**Why the order of groups does not matter.** At the end of a group, the bounds of its own variables come from its own code, from new instances of general schemes, from closed types, and from pinned variables, which are closed types. The only other source, an earlier variable that is not pinned, makes the group open (F3). So the pins, read types and errors of a group do not depend on the order of the groups that are not its ancestors. A later group can only meet a pinned variable as a closed type, so its errors are its own.

**What pinning costs.** A pin restricts later code as a written type would. With `Cell(A) { Cell(get: () -> A, set: (A) -> Nil) }`, `r = cell(1)` and `C = r`, the variable of `r` is pinned to `Int`, so a later `r.set("s")` is E0501 with the note above. Without `C`, `r` would get the type `Cell(Int | String)`. This is the same as writing `r: Cell(Int) = cell(1)`, and the note tells where the type was fixed.

**Example.** In records.md 5.9, `user` is a call, so it is not generalized. Its type is `Codec(D, E)`. `field` returns `Codec({ ..R, K: V }, E & { K: V })` and takes `Codec(V, V)`, so the `V` of each field has the lower bound and the upper bound `String` or `Int`: its fixed form is that type. `D` has the lower bound `{ age: V2, name: V1 }`, so `fixed(D) = { age: Int, name: String }`. The pattern `Codec(User, _) = user` reads only the decode part, so `D`, `V1` and `V2` are pinned, and `User = { age: Int, name: String }`. `E` is not read and not pinned: later `user.encode(u)` still works. `Codec(_, Enc) = user` would read `E`, which has only upper bounds (nothing has come to it), so it is E0534: "nothing has come to this part of the type of `user` yet; write the type of `user`".

**Properties that the tests check.** For every top-level `x` whose read type is closed, `x`'s type and its read type are equal, because every variable in it is pinned to its fixed form; so `check_x: x = x` is accepted. Typing the groups in another topological order (a debug option of order.rs that breaks ties the other way) gives the same read types and the same errors.

### 3.4 Order and cycles

- **O1. Type needs of definitions.** In resolve, after `alias_circles` (:1175), compute for each `TypeId` the set `value_deps[t]` of top-level `DefId`s that the definition reads: by `Of` in its body, by an alias pattern's source, and, transitively, through the aliases it names (alias bodies and alias-pattern heads). Aliases have no cycles once the circular ones are removed, so a memoised DFS computes it in O(aliases + edges). `uses_value_type[t]` is `!value_deps[t].is_empty()` or "names a declaration or a value of another module". Most entries are empty.
- **O2. Type uses of a binding.** `type_uses(b)` is the set of top-level bindings `d` of the same module such that a type written anywhere in `b` has `Of(Def(d))` or names an alias `t` with `d ∈ value_deps[t]`. The walker is one generic visitor, `hir::for_each_type_in_binding`, that visits every `hir::Type` in a binding: typed patterns (bindings, parameters, clause patterns, type tests), `->`, `?`, `!` parts, env params and `Stmt::Env` (they hold no `Of` after E0531, but they may name aliases), and every inner binding statement. Both order.rs and the debug check of O6 use this one visitor, so no kind of place can be missed by one and seen by the other. The Lisp surface lowers to the same core tree and HIR, so it needs nothing more.
- **O3. Groups.** Groups are the SCCs of the **use edges only**, as today (order.rs :188-228). Then a second graph is made over the groups: an edge for each use between groups and for each type use between groups. Groups are typed in a topological order of that graph.
- **O4. E0530.** A type use whose two ends are in one group (a self loop included, through an alias), or a cycle of groups in the second graph, is **E0530**. The message names the chain: "the type of `num` is used in a type in `check` (through the alias `N`), but `num` and `check` are typed together: `num` uses `check`. A type may only use the type of a value that is already typed. Write the type, or move the use out of this group." The type uses in the cycle are dropped for ordering, and the sites that read them are marked `Failed`: definition sites and binders in `def_reads` and `bound`, annotation sites in `site_failed` (part 3.6). A `Failed` site gives `Error` with no second message. So in `a = (n: b) { n }` with `b = a(1)`, the site `b` in `a` is `Failed` before `a` is typed, and `conv` gives `Error` for it, not the internal error of O6.
- **O5. Groups do not change.** A correct program has no E0530, so the groups are those of today and only their order is fixed. Needs, init order and E-needs-itself (`needs()`, order.rs :137) are computed from `uses()` only and do not change at all. A unit test compares the groups with and without type edges.
- **O6. No read before typing.** A read of a top-level value that is not typed yet, at a site that is not `Failed`, is an **internal error**, never a fresh variable, when no error was reported before inference. When resolve or order reported an error (an import circle E0214, E0530, E0531, or any other), such a read is marked `Failed` and gives `Error` with no message: `check_for_queries` runs inference after errors (under `catch_unwind`), and one bad read must not lose the types of the whole program for the editor. In `conv`, an `Of` never reaches the `alias_depth > 64` fallback (infer.rs :858), and in `kconv` never the `Top` fallback (known.rs :108). A debug assertion, on only when no error was reported before inference, checks that every annotation-reading `Of(Def(d))` finds `typed[d]` or a `Failed` mark.
- **O7. Locals and other modules.** Locals need no edges: statements are typed in order, and V4 covers the cases that would read too early. Values of other modules are typed first (`module_order`, order.rs :255), so their read schemes exist, except in an import circle (E0214), where `module_order` puts the modules of the circle in id order. `i check` stops before inference after E0214; the query path does not, and O6 makes those reads `Failed`.
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
  - or an alias whose expansion, through aliases, is a single named custom or built-in type or a tuple type, which does not use a value type (else **E0532**). Its body is put in, with its parameters replaced by the parts of the head. Each parameter must be used at most once in that body and only at a place that is a pattern place (an argument of a named type or a part of a tuple); else E0532 with the hint "the alias `MyCodec` stands for `Codec((D) -> Int, D)`; write the custom type it stands for".
  - The tuple case is how a top-level tuple is taken apart. A tuple has no name to write as a head, and an item that starts with `(` is a binding (P6), so `(Tree, _) = parsed` is a binding with a tuple pattern. With `Pair(A, B) = (A, B)`, the item `Pair(Tree, _) = parsed` takes the type of `parsed` apart. No new syntax is needed. When a binding whose pattern is a tuple of `UpName`s and `_` gets an error and its value is a lone value name, the message gets the hint: "to take apart the type of `parsed`, write an alias such as `Pair(A, B) = (A, B)` and then `Pair(Tree, _) = parsed`". A function type has no such alias form, because a parameter of a function type is not a pattern place (open question 4). The parts of the body with no parameter become **exact parts**: the part read there must be closed and equal to them (`τ <: T` and `T <: τ` on closed types), else E0535.
- **P3. Parts (TypePattern).**
  - A plain `UpName` (no module, no arguments) is a **binder**. It defines a type of the module with that name. A second definition in the module is E0208. The same binder twice in one pattern is E0208. A binder with the name of a type that may be written with no module name (imported or built-in) is **E0537**: "`Int` is a type; a name in an alias pattern defines a new type; use `_` to skip a part".
  - `_` is a **skip**: nothing is read or checked there, and that part may hold general variables or unfixed parts. It is `TypePat::Skip`, never `TypeKind::Hole`.
  - `m.T`, `T(P, …)` and `m.T(P, …)`, where `T` is a custom or built-in type, take that type apart. An alias there follows P2.
  - `(P1, …, Pn)` takes a tuple apart.
  - Anything else is **E0532**: a union, an intersection, a difference, a literal, a function type, a record type (T3 may add record patterns), a value type.
  - A macro call in a part is not in this list: the head is a type position, so macro step 1 (meta.md §4.4) expands it before P1, and its result is matched as a pattern. `Codec(pick(User), _) = user`, where `pick` returns its argument, is `Codec(User, _) = user`. A binder that comes from a macro's template is a type name defined by the macro, with the hygiene of meta.md §5. If the result is not a pattern form, it is E0532 at the call, and the message says what the macro gave.
- **P4. Matching.** Let τ be `read[x]` (the definition reading of the source), with no closedness required yet. Then, from the top:
  - at a named pattern `C(P1 … Pn)`: τ, made simpler and with aliases at its top expanded, must be exactly one member, a named type with the type constructor of `C` and n arguments (not a union, intersection or difference). Each `Pi` is matched with the i-th argument.
  - at a tuple pattern: one member, a tuple with the same number of parts.
  - at an exact part (P2): equal, as above.
  - at a binder: the part must be closed (E0533 or E0534). The binder becomes a **structural alias** of that closed type. It has no parameters and no constructors (`User(..)` in the exports is the existing error for aliases).
  - at a skip: nothing.
  - a general variable or an unfixed part where a named type or a tuple is needed: E0533 or E0534. Anything else: **E0535**: "the type of `n` is `5`, which does not match `List(E)` (§4.3)".
- **P5. When.** Each pattern, and each definition-reading site, is handled **once**, at the end of the group that types the last of its value deps. Sites whose deps are all in other modules, declarations or built-ins are handled by a **module-start pass**, which runs for every module in `module_order`, whether or not the module has any group. (A module with no top-level bindings, such as one that only has `Codec(User, _) = m.user`, has no groups: `Checker::run` only calls `group` for `p.order.groups`. So the pass is a step of the module loop in `run`, not of `group`.) At each of these points, items are handled in source order. The export check (part 3.8) runs at the same point. So the errors come at the definitions, in a fixed order, whether or not anyone reads the binder.
- **P6. Top level only.** Blocks have no type definitions, and an item that starts with `(` or `{` is a binding.
- **P7. Two meanings of `_`.** On the left of an alias pattern, `_` skips a part. Everywhere else in a type it is a hole that inference fills. A skip never reaches `show_type`, the interface, or `TypeKind::Hole`.

### 3.6 The cache and the cost

- `program.has_value_types` is true when the program has any `Of` or alias pattern. When it is false: order.rs does not walk types, `group` pins nothing, the solver sets no `group_open` flag and `pinned` stays empty, no tables are filled. Edge counts and std check time do not change (a check in the tests).
- `program.read_targets: HashSet<DefId>`: every def named by an `Of` or a pattern source, in any module. Only these get `read[x]`, and only their read parts (`program.read_parts`, F2) are pinned.
- `Checker.def_reads: Vec<DefRead>`, indexed by `OfId` for definition sites, and the binders' entries in `Checker.bound: HashMap<TypeId, DefRead>`. `DefRead` is `Pending | Closed(Ty) | Failed`. P5 fills them. A use in `conv`, `kconv`, `expand` or `check_places_ty` clones a `Closed` entry. `Failed` gives `Error` with no new message. `Pending` gives `Error` when an error was reported before inference (O6), and otherwise an internal error (and a debug panic).
- `Checker.site_failed: BitSet` over all `OfId`s, for annotation sites, which have no `DefRead`. It is set by O4 (a site in a cycle) and by O6 (a read of a value not typed after an earlier error). `conv` and `kconv` check it before they look at `read[x]`, and give `Error` with no message.

### 3.7 Known types, places, literal types

- **K1. Known type** of a value type (`kconv`, known.rs :102):
  - top-level, imported, a declaration, a built-in, or a finished generalized local: `read[x]` (or the instance) with general variables, `Top` and unfixed parts as `_`;
  - a name still being worked out (A3): the known type that `fit_known` (known.rs :457) gave that name from its own pattern, which is the part of the written type at the name's place (for `(a, b): (Int, String)`, `a` gets `Int`), and `_` when the name has none. So in `(p: User) { q: p = p ⏎ q.name }`, `q` has the known type `User`, and `q.name` works; in `((a, b): (Int, String)) { q: a = a ⏎ q == 1 }`, `q` has the known type `Int`, not `(Int, String)`, so `q == 1` is not E0520.
  - a definition reading: the known form of the `Closed` entry.

  It is never the known type of the name. After `x = "a" = s`, the name `x` has the known type `"a"`, but `y: x = t` lets any `String` in (the type of `x` is `String`), so the known type of `y` is `String`, and `y == "b"` is not E0520.
- **K2. Literal types.** The type is read with no widening: `port = 8080` then `Port = port` makes `Port` the type `8080`. To get `Int`, write the type at the source (`port: Int = 8080`). The same holds for records: `p = { name: "a" }` gives `{ name: "a" }`.
- **K3. Places (§4.4, E0518).** `check_places` walks HIR types and treats `Of` as opaque. A new `check_places_ty(&Ty, Pol)` checks the read type at the polarity of the place where `Of` is written, before general variables and `Top` are replaced: a general variable counts as a written type variable, and `Top` as `_` (which is allowed anywhere). `use = (f: pick) { f(1, 2, True) }`, with `pick: (A, B, Bool) -> A | B`, is E0518, because `(f: (A, B, Bool) -> A | B)` would be. A closed type always passes.
- **K4. Type tests.** `x: user` in a clause pattern is allowed. `Typed.type_tests` heads come from the read `Ty`, so lower.rs needs no change. A debug assertion checks that a `Head` is never a binder `TypeId`, because `head_cond`'s alias fallback would give `Tag(u32::MAX)`.

### 3.8 Exports, the interface, queries

- **X1. §2.4.** An exported binding whose top annotation uses a value type, directly (`Of`) or through an alias `t` with `uses_value_type[t]` (also a private binder, such as `y: Inner` with `Box(Inner) = b`), counts as not written (`self.written`, infer.rs :85, :989), so its inferred type goes through `exported_type` (:569) and EXPORT_HIDDEN_TYPE. An exported binder, and an exported alias with `Of` in it (directly or through aliases), is checked at P5 on its read type with the same rule. The message names the source: "`User` is exported, but the type that `Codec(User, _) = user` gives it uses `Secret`, which is not exported". `hidden_types` (resolve.rs :2460) skips `Of` and these aliases, so infer must check every annotation that reaches them; the test for `uses_value_type` through aliases is the same for exported aliases and for exported bindings.
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
| E0534 | VALUE_TYPE_NOT_FIXED | an unfixed variable is read: nothing has come to it yet, its form holds itself, or its group met an earlier variable that is not pinned (F3, F4) |
| E0535 | TYPE_PATTERN_MISMATCH | the type does not match the pattern, or an exact part differs |
| E0536 | VALUE_TYPE_OWN_SCOPE | V4: same pattern, same parameter list, own value |
| E0537 | TYPE_PATTERN_BINDER_IS_TYPE | a binder has the name of an imported or built-in type |

Codes used again: E0201, E0203, E0204, E0205, E0207, E0208, E0221, E0227, E0501, E0518, TYPE_ARITY, ALIAS_CIRCLE. A3's code for "a lowercase name in a type" is removed with its tests in the same change; those tests become value-type tests.

## 5. Why it is sound (summary)

- In the annotation reading, a value type is only another source of constraints, like `_` holes linked to each other. Holes and rule 3 are already sound.
- In the definition reading, the type is closed, so every use of an alias means the same type.
- Trusted places are excluded (E0531).
- A value is never read before it is typed (O6), and E0530 catches every cycle.
- Fixed types do not go stale: a read variable is pinned, and the solver reads it as its closed type from then on (F5). This does not rest on where variables sit, so service arguments compared both ways and the checks of `add_bound` cannot break it.
- The result does not depend on the order of groups that are not ancestors of each other: a group that met an earlier variable that is not pinned gives no fixed type (F3).
- Principal types are kept: every value type is a function of a principal scheme found earlier, or of a variable shared inside the same binding (like a declared type variable). So the binding that uses it is typed by plain MLsub.

## 6. Records 5.9, step by step

```i
user = object |> field("name", string) |> field("age", int)
Codec(User, _) = user
show = (u: User) { u.name }
```

1. resolve: P1 makes the item a pattern; `Codec` is a custom type of arity 2; `User` is a binder; `_` is a skip. `value_deps[User] = { user }`. `type_uses(show) = { user }`.
2. order: `show` is typed after `user`'s group; groups are unchanged.
3. infer: at the end of `user`'s group, F2 says only the decode part is read. F4 gives the `V` of each field its one bound (`String`, `Int`), and `D` the form `{ age: Int, name: String }`; they are pinned. `E` is not read, so it is not pinned, and `user.encode` keeps working. P5 matches the pattern: `User` is `Closed({ age: Int, name: String })`.
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
> - **Alias pattern.** In `T(P1, …, Pn) = v`, `T` is a type that already exists, written with no module name; it is not defined by the item. (`a.T(…) = v` is a binding, §2.2; to take apart a type of another module, import its name.) `T` is a custom type or a built-in type with n parameters, or an alias that stands for one such named type or for a tuple type. The item takes apart the type of the value `v` (§4.7). Each `Pi` is a type pattern:
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
>   Pair(A, B) = (A, B)
>   Pair(Tree, _) = parsed    // with parsed: (Ast, List(Warning)), Tree is Ast
>   ```
> - A name defined by an alias pattern is an alias: it has no parameters and no constructors, and it is exported as `T`.
> - Custom types are **nominal**: two different definitions make two different types, even if they have the same form.

### 7.5 §4.7 Type inference (add after the bullet on groups)

> - **Types of values.** A type of a value `x` (§3) is read from the type that inference finds for `x`.
>   - **In an annotation.** In a type written in a binding (on its pattern, in parameters, `->` parts, clause patterns, and inner bindings), `x` has the type that the name `x` has as an expression at that place (§4.8 rule 3). Each general type variable of `x` becomes a new type variable, one for each variable, at each place where `x` is written, as `_` does. For example, with `id = (x) { x }`, `twice = (f: id, n: Int) { f(f(n)) }` gives `f` the type `(A) -> A` for a new `A`.
>   - **A name being worked out.** A parameter, a name of a clause pattern, or a local binding that is not generalized has one type, and the annotation shares it. So in `(xs: List(Int)) { acc: xs = [] … }`, `acc` and `xs` have one type: the value of `acc` must fit in the type of `xs`, and what code asks of `acc` it asks of `xs`.
>   - **In a type definition.** In an alias or an alias pattern (§4.3), the type must have no type variable and no `_` in the parts that are used. `Same = id` is a compile error; `Codec(Size, _) = size` is correct.
>   - **The fixed type.** A top-level binding that is not generalized is read through its **fixed type**: the type that is shown for it (the bullet of this section on how types are shown: a type variable that is not generalized is shown as the union of the types that come to it), taken when its group is typed. Each type variable in a part that is read becomes the union of the types that have come to it. Example: with `user = object |> field("name", string)`, the decode side of the type of `user` is `{ name: String }`.
>   - **Reading fixes the type.** From then on, the binding keeps that type in the parts that are read, as if the type had been written at the binding. Later code that would give such a part a wider type is a compile error, and the message says where the type was fixed. Parts that no type reads, such as a part under `_` in an alias pattern, are not fixed.
>   - **Not fixed.** A part is not fixed when no type has come to its type variable yet, when its type would hold itself, or when the binding's group uses a part of an earlier binding that is not fixed. A type that reads such a part is a compile error. Write the type of the binding. Examples: `g = id(id)` is not generalized, and nothing has come to the `A` of its type `(A) -> A`, so `(h: g)` is a compile error. With `user` as above, `Codec(_, Enc) = user` is a compile error, because no value has yet been given to the encode side.
>   - **Why fixed types do not change, and do not depend on order.** A fixed part is a type with no type variable, and every later use is checked against it. Its value depends only on the binding's group and the groups it uses, never on other groups typed before it.
>   - The type that is read must keep the rules of places of §4.4 at the place where it is put, as a written type must.
> - **Order.** A type of a value is read when the value is typed. A binding whose types use the type of a top-level binding of its module, directly or through an alias, is typed after the group of that binding. A type never puts two bindings in one group: when the types of a binding use the type of a binding of its own group, or its own type, it is a compile error.
> - The value restriction is the same for types of values: a binding that is not generalized gives no general type variable, and a type of a value never makes a value general.

### 7.6 §4.8 Typing rules (edits)

- In the pattern types, "`x: T`: `T`, with a new type variable in place of each `_` in it" becomes "`x: T`: `T`, with a new type variable in place of each `_` in it, and each type of a value in it read as §4.7 says".
- In **Names in patterns**, after "The name `x` of `x: T` gets `T`", add:

> A type of a value `y` in `T` gives, as a known type, the type that §4.7 reads for `y`, with `_` for each type variable in it. For a parameter, a name of a clause pattern, or a local binding that is not generalized, it is the known type that the name gets from its own pattern (the part of the written type at the name's place), or `_` when there is none: in `(p: User) { q: p = p … }`, `q` has the known type `User`, and in `((a, b): (Int, String)) { q: a = a … }`, `q` has the known type `Int`. It is never the known type of the name `y`, which may have fewer values: after `y = "a" = s`, `z: y = t` gives `z` the known type `String`, not `"a"`.

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

- **ilang-syntax** (after A3): `ast::TypeKind::Value` exists. The A3 error for a lowercase name in a type is removed. The A1 error for an alias whose head is not a list of type parameters ("the head is kept for alias patterns", syntax.md §4.3 and the notes for A1), which the parser reports and resolve reports again for a part that names a type, is removed in both places: P1 classifies the item, and a head on a right side that is not a value type gets the type-parameter messages of the new §4.3 ("a type parameter is an `UpName`", "a type parameter may not have the name of a type"). `type_vars()` never collects `Value`. The P1 classification is made on the core tree; the Lisp printer and reader get the rows of 3.9. The `json.Codec(…) = user` hint is added where the binding's error is reported (resolve).
- **resolve.rs:**
  - `collect_type` (:528): P1. For a pattern, register each binder as a `TypeDef`, with E0208 and E0537; do not register the head.
  - `type_defs` (:1098): resolve the head (P2) and the parts (P3, E0532), and the value name to a `ValueRef::Def` (DefIds exist in phase 3; scope values and `defs_by_name`).
  - `ty` (:1329): a `Value` arm. In annotations, `find` (:1834) with V4 (a `pending_binders` stack checked first, E0536); in definitions, the phase-3 lookup. `a.x` through `member` (:1853), E0207 when `a` is a local in scope or on `pending_binders` (V3).
  - `function` (:2133) and block statements: push the names being bound onto `pending_binders` while their types are resolved; mark the own value of a binding.
  - `alias_circles` (:1175): binder edges to the head and the parts (O8). After it: `value_deps`, `uses_value_type` (memoised DFS), `read_targets`, `has_value_types`.
  - E0531 checks, using `uses_value_type`, in constructor fields, `external_type`, `env_type` (:1445), `host_part` (:1568), `host_fn`, `host_type`. A hit is E0531 only, not ENV_TYPE or EXTERNAL_HOST_TYPE.
  - Exhaustive arms: `named_types`, `for_each_child`, `subst` (`Of` stays), `hidden_types` (:2460, skip), `expand_alias` (:1412, never reaches env and host paths after E0531), `type_vars` (never collect).
  - `type_value_names` per binding.
- **order.rs:** `type_uses` with the shared visitor; the group graph of O3 next to `tarjan_scc` (:220); E0530 with a BFS chain; `needs()` (:137) and init (:230) unchanged; the unit test of O5.
- **types/infer.rs:**
  - `Checker`: `first_var`, `unfixed: HashSet<V>`, `read: HashMap<DefId, Ty>`, `def_reads`, `bound`, `site_failed`.
  - `group` (:233): F1–F6 at its end, then P5 for the sites and patterns it completes.
  - `run`: the module-start pass of P5, in the loop over `module_order`, for every module, also one with no groups.
  - `read_value_type(site, level)`: A1–A3 and the definition reading.
  - `conv` (:852): an `Of` arm; no `alias_depth` fallback for `Of` (O6).
  - `expand` (:953) and `check_places` (:603): `Of` is opaque; `check_places_ty` (K3).
  - `walk_variance` and `variances`: `Of` adds nothing (it never appears in a field, E0531).
  - P4 matching (a new `alias_pattern.rs` next to `infer.rs`), E0533–E0535.
  - `written` (:989) and `exported_type` (:569): X1. `Typed.alias_types`.
- **types/solve.rs:** `pinned: HashMap<V, Ty>` and the replacement of a pinned variable in `constrain_in`, `reach` and `compare` (F5); the `group_open` flag and `first_var` (F3); the note on failures against a pinned type; the debug assertions of 3.3 (no bound pushed onto a pinned variable, the pin's own constraint gives no failure).
- **types/known.rs:** `kconv` (:102) arm (K1), which takes the name's known type from `fit_known` for A3 names.
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
4. resolve.rs: P1–P3 and binders, the `Value` arm, `a.x`, V4 and E0536, alias graph edges, `value_deps`, `uses_value_type`, `read_targets`, E0531, the `json.Codec` hint, the T3 hook. Remove A3's lowercase error and its tests, and the A1 "kept for alias patterns" error in the parser and in resolve with its tests; those tests become alias-pattern tests or get the new type-parameter messages.
5. order.rs: type uses with the shared visitor, the group graph, E0530 with the chain. Unit test: groups, needs and init are unchanged with and without type edges.
6. solve.rs first: `pinned`, `group_open`, and their unit tests. Then infer.rs and known.rs: read parts, fixed forms and pins (F1–F6); A1–A3 and the definition reading; `conv` and `kconv` with `site_failed`; `check_places_ty`; P4–P5 with the module-start pass in `run`; the caches; the exported checks (X1 through aliases).
7. `Typed.alias_types`; query.rs; package.rs.
8. Spec text (part 7) and the doc edits of 8.2.
9. Tests (part 10). Run `check-all.sh` and the std checker. Confirm no change in goldens without value types, and that std check time is unchanged within noise (S2's script).

## 10. Tests

**tests/spec:**

- `3/value_type.i` (run): `twice = (f: id, n: Int) { f(f(n)) }`; `(xs: List(Int)) { acc: xs = [] … }`; `{ x }` is `x`; `m.x` from an imported module.
- `3/value_type_errors.i`: `x(Int)` in a type is still the macro error; a local `a` covering module `a` in `a.x` is E0207 with the field message; `(p: User, q: p.name)` and `(p: User) -> p.name { … }` are E0207, also with an imported module `p` that exports `name`; `m.private` is E0204.
- `3/value_type_own_scope.i`: E0536 for `(x, y: x)` with a top-level `x` (the message names it), `(x) -> x { x }`, a block `x: { x } = 1` with an outer `x`, a clause pattern `{ (a, b: a) -> … }`, and a local `f = (g: f) { g }`.
- `3/value_type_places.i`: E0531 for a constructor field, an alias used in a constructor field, `@external` with `(user) -> Int`, `! T = e`, `x ! T`, a host service type.
- `4.3/alias_pattern.i` (run): the Codec example (`Size` is `Int`, `(n: Size)` accepts `1`); `List(E) = xs`; `Pair((A, _), _) = p`; `Codec(_, _) = size`; `T = size_fn` (plain); a head alias `MyCodec(D) = Codec(D, Int)` with `MyCodec(S) = size`; a tuple through a head alias, `Pair(A, B) = (A, B)` with `Pair(Tree, _) = parsed`; a macro in a part, `Codec(pick(User), _) = user`, defines `User`.
- `4.3/alias_pattern_errors.i`: E0532 for `Codec(Int | String, _) = size`, for a head alias `F(A) = Box((A) -> Int)`, for a function-type part; E0535 for `List(E) = n` with `n = 5`, and for an exact part that differs; E0533 for `Same = id` and `Codec(D2, _) = poly`; E0537 for `Codec(Int, _) = size`; E0208 for `Pair(A, A) = p` and for a binder that is also a custom type of the module; ALIAS_CIRCLE for `Schema(A, B) = Codec(User, B)` with `Schema(User, _) = user`.
- `4.3/alias_pattern_qualified_head.i`: `json.Codec(U, _) = u` parses as a binding, and its error has the import hint; `(Tree, _) = parsed` is a binding, and its error has the `Pair` hint of P2.
- `4.3/alias_head_old_error.i`: `Pair(Int) = (Int, Int)` gives the message that a type parameter may not have the name of a type, and `Box(List(A)) = …` the message that a type parameter is an `UpName`; the A1 "kept for alias patterns" message never appears.
- `4.3/alias_pattern_no_bindings/` (project): a module with no top-level bindings, only an import, `Codec(User, _) = m.user` and `Size = m.size`, and the export `(User)`; another module uses `types.User` and `types.Size`. The binders are read and the export is checked, with no internal error.
- `4.3/alias_head_import/` (project of two modules): `Pair(A) = (A, A)` is correct; after importing a type `A`, it is the type-parameter error, not a pattern. `Codec(User, _) = user` is correct; after importing a type `User`, it is E0537, not a test against `User`.
- `4.3/alias_pattern_unread.i`: a pattern that nobody reads still gives its E0535, at the item, in source order.
- `4.7/value_type_order.i`: a reader written before its source is typed after it; the init order is unchanged (prints show the source order); no E-needs-itself from type uses alone.
- `4.7/value_type_circle.i`, E0530: `x: T = 1` with `T = x`; `a = (n: b) { n }` with `b = a(1)` (one E0530, and `a` is typed with `Error` for `n`, no internal error); `user: Codec(User, _) = …` with `Codec(User, _) = user`; a chain through two aliases (the message names it).
- `4.7/value_type_fixed.i`: `Codec(Size, _) = size` gives `Int`, and `size.encode(5)` still works after it (the encode side is not read, so not pinned); `(c: size)` is E0534, since nothing has come to the encode side; `check: size = size` is accepted; `g = id(id)` used in `(h: g)` is E0534; `Codec(_, Enc) = user` is E0534 with "write the type of `user`"; the records.md 5.9 `user` gives `User = { age: Int, name: String }` (each field's `V` has one lower and one upper bound).
- `4.7/value_type_pin.i`: with `Cell(A) { Cell(get: () -> A, set: (A) -> Nil) }`, `r = cell(1)` and `C = r`, a later `r.set("s")` is E0501 with the note that `C` fixed the type of `r`; without `C`, the same file is accepted. The services case: with `State(S) { State(get: () -> S) }`, `get_s = (st ! State(S)) { st.get() }` and `reader = id(get_s)`, `R = reader` is E0534, and `use = () { ! State(Int) = State(get: () { 5 })  reader() }` and `check: reader = reader` are accepted in either order. The `compare` case: `Box(A) { Box(get: () -> A, put: (A) -> Nil) }`, `p = { b = make_box()  (b, () { unbox(b) }) }` with `P = p`, and a later use of `p.0` where `Box((γ) -> Int)` is expected; it is accepted, and `P` stays `(Box((String) -> Int), () -> (String) -> Int)`.
- `4.7/value_type_fixed_shared.i`: variables shared across groups and across modules: `a = ref_like()` in module m, `b = wrap(m.a)` in module n, `B = b` is E0534 (the group of `b` met the variable of `m.a`, which is not pinned); the same with `A = m.a` also in the program pins `m.a` first, and then `B = b` is read. Run with the order.rs debug option that breaks ties the other way: the same errors and read types.
- `4.8/value_type_known_parts.i`: `f = ((a, b): (Int, String)) { q: a = a  q == 1 }` and the clause form `v |> { (a, b): (Int, String) -> { q: a = a  q == 1 } }` give no E0520 and no E0501.
- `4.7/value_type_import_circle/` (project, query test): `app/a` and `app/b` import each other, `a` has `T = b.x` and `y: T = 1`. `i check` gives E0214 only; `check_for_queries` gives types for the rest of the program (hover works), with no panic.
- `4.7/value_type_places.i`: `use = (f: pick) { f(1, 2, True) }` is E0518.
- `4.8/value_type_known.i` (run): `(p: User) { q: p = p ⏎ q.name }`; after `x = "a" = s`, `y: x = t` then `y == "b"` gives no E0520; `port = 8080 ⏎ Port = port ⏎ bad: Port = 8081` is E0501.
- `2.4/export_value_type/`: an exported binder bound to a private custom type is E0227 with the source in the message; an exported `y: secret_val` whose type uses a private type is E0227; an exported `y: Inner = Secret(2)` with a private binder `Box(Inner) = b`, `b = Box(Secret(1))` and a private `Secret` is E0227; a private source with only exported types is fine.

**Checker goldens** (`crates/ilang-check/tests/types/value_types.i`, `alias_patterns.i`): the shown schemes of readers, the read types with UpName variables.

**Unit tests:** order.rs, groups equal with and without type edges and needs/init unchanged; fixed forms and pins on hand-built solvers (recursive bounds, variable bounds with no lower bound, a pinned variable met by `constrain_in`, `reach` and `compare`, env arguments, `group_open` set by an earlier variable that is not pinned and not set by a pinned one); `alias_pattern.rs` matching; the assertion fires on a bound pushed onto a pinned variable by hand.

**Query tests:** hover on `size` in a type shows its scheme; hover on a binder `User` shows `User = { … }`, and the written pattern when inference fails; go to definition from `User` goes to the binder, and from `user` in a type to the binding; the outline lists the binders.

**Package test** (`crates/ilang-cli/tests/package_commands.rs`): changing a private `size` to give `String` reports `type m.Size` as changed; an exported alias `T = List(x)` is compared by its read body; nothing changes when nothing changed.

**Property test:** for every top-level binding of std whose read type is closed, a generated `check_x: x = x` is accepted. With the debug solver assertions on, the whole std and the spec tests run with no assertion failure.

**Performance:** std check time unchanged within noise; edge counts equal for programs with no `Of`.

## 11. Risks

- **E0531 on constructor fields** limits nominal wrappers such as `Account { Account(User) }` with `User` from a pattern. Users must use aliases (or records after T3). Lifting it needs a sound order for field access by label.
- **Reading fixes the type.** A pin restricts later code as a written type would (part 3.3, "What pinning costs"). So adding `C = r` in one module can make code that widens `r` in another module an error. The error is deterministic and its note names the reading site. It is a new kind of action at a distance, but the same as writing the type at `r`.
- **Pins in the solver are new code with soundness weight.** Every path that walks bounds (`constrain_in`, `reach`, `compare`, `extrude`, `freshen`, display) must replace a pinned variable. A missed path could let a pinned variable gain a bound. The assertion in `add_bound` and the tests of `value_type_pin.i` target this.
- **Open groups are coarse.** A group that meets any earlier variable that is not pinned gives no fixed type for any of its read targets, also for parts that do not depend on that variable. The fix is to write a type; a finer rule (per connected part) can come later without changing correct programs.
- **check_places_ty** checks solver types, not written ones. Read types simplified differently from written types (the `has_both` fallback in `generalize`) may be rejected or accepted in surprising ways.
- **The head needs an `UpName` with no module name.** `json.Codec(User, _) = user` is a binding. The hint helps, but a correct binding of that form gets no hint.
- **Literal types are not widened.** `Port = port` is `8080`, and through exported aliases a private value's body change can change the interface. Intended, but a new source of interface churn.
- **Head aliases** (P2) add exact parts and an equality test on closed types. It is a small new piece of the matcher.
- **Codes.** If T1 needs more than seven codes, or the split is not confirmed, the codes clash on merge.
- **Order with T3.** T3's resolve-time checks must walk `Of` (the hook). The plan keeps T2 before T3, and there are no key parameters before T3, so there is no gap.
- **A3 and A2 prerequisites.** Without UpName shown variables, hover and interface text read as value types (step 1 stops).
- **Nothing has come yet.** A part with nothing coming to it is E0534, not `Never`. So `(c: size)` is an error while `size.encode` has never been called, and a call added later elsewhere does not fix it (the read happens at the end of `size`'s group). The message says to write the type.

## 12. Open questions

1. Should value types be allowed in constructor fields later, with a "prelude group" rule that types the sources before the custom type is used?
2. Answered: a part with nothing coming to it is E0534, never `Never` and never the types it goes to (part 3.3). Open: should a part with only upper bounds be read as their intersection when the user asks for it, for example with a later form for input types?
3. Left-to-right scoping of parameters in types (`(a, b: a) -> a`) as a later extension. It cannot change meaning, because today it is E0536.
4. Function-type patterns and record patterns (T3) in alias patterns. A top-level tuple is taken apart through a head alias (`Pair(A, B) = (A, B)`, P2); a function type has no such alias, so taking apart a function-typed value needs a function-type pattern.
5. Should matching relax "exactly one member" for a union with `Nil`, as known-type matching does?
6. A lint for a correct binding `a.C(…) = x` that looks like an alias pattern.
