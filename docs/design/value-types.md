# Value types and alias patterns (T2)

Status: revised design for G2/G3, not an implemented feature. This note replaces the interrupted S6 draft. T2 installs §7 in the spec before implementation. All examples use the accepted syntax. Code references below were checked in this lane at `f745f9fa3d8f59f371c422f0c9dcfed763cfca42`; numbers refer to that tree, before A1/A3/T1.

## 1. Problem and sources

A library should be able to build a value and let its caller name a part of its type:

```i
user = object |> field("name", string) |> field("age", int)
Schema(User, _) = user
```

This is type checking, with no execution of `user`. `User` names a set of values, not a new nominal type. The same features support a schema's decoded type, a parser's result type, and a callback annotation that follows another function's signature. Literal constants and protocol descriptions are further uses. None needs a compiler rule for that library.

Controlling sources, in order:

- `docs/spec.md`: §2.4 exports, §2.5 runtime needs, §4.3 transparent aliases, §4.4 places, §4.7 principal inference and the value restriction, §4.8 schemes and known types, §5 scope.
- `docs/design/syntax.md`: §3 reserves lowercase types and lowercase calls separately; §4.3 deliberately leaves alias-pattern classification to T2; §4.7 requires declared UpName variables.
- Accepted `docs/design/literal-types.md` decisions 4, 6, 7 and its §4.8 replacement: no widening; a known type can differ from the inferred type.
- Accepted `docs/design/records.md` §§5.9, 8.2, 11, 15.6: decode-side extraction, groups versus records, label restrictions, canonical interfaces. The ledger accepts its check of places on shown monomorphic types.
- Plan `/home/agent/.claude/plans/twinkling-swinging-perlis.md`, “已定的设计” and T2: value types, structural extraction, UpName variables, no cycles; A3 → T2 → M2 in resolution/order, T1 → T2 → T3 on types.
- Binding ledger `/home/agent/Programming/i/.rd/cache/ledger.md`: lowercase head followed by `(` means a macro call; one-part braces group a type unless the record syntax applies.
- Macro draft read with `git show lane/note-meta:docs/design/meta.md`, at `44021cc52e7fc0d3b3c6646067d29f81157c577d`: §§2.3, 4.4, 5, 6, 7.3–7.4. It is an integration contract proposed here, not a source that overrides the accepted notes.

The difficult part is a value whose scheme contains shared, non-generalized inference variables. Reading a temporary display of those variables as a permanent alias is wrong. The earlier draft tried to solve this by replacing variables inside the solver. This revision keeps the explicit fixation rule but implements it with ordinary checked constraints (§4), preserving their existing relationships.

## 2. Code checked, and implications

These are facts from reading code, not claims that T2 already works.

| Code and lines | Fact | Consequence for T2 |
|---|---|---|
| `crates/ilang-check/src/resolve.rs:44–63`, `:421–448`, `:528–592` | Names/exports and imports are collected before type bodies, and type bodies before values. | Collect pattern binders early; resolve a value reference without inferring it. |
| `resolve.rs:1098–1147`, `:1175–1209`, `:1329–1379` | Alias bodies remain HIR types; alias cycles follow named aliases. | Add `Of`, and alias-pattern edges, rather than eagerly converting bodies. |
| `resolve.rs:1834–1849`, `:1853–1888`, `:2138–2202` | Value lookup checks locals before module scope; members check exports; parameter and result types are resolved before parameter locals are pushed. | Use the ordinary value lookup, plus an explicit pending-name check to prevent accidental outer capture. |
| `resolve.rs:1445–1467`, `:1568–1600` | Service heads and host forms are inspected during resolution. | Reject value-dependent aliases there before those checks. |
| `crates/ilang-check/src/order.rs:17–26`, `:36–85`, `:188–251` | The use walker visits patterns/expressions, not their types; expression uses form SCCs; initialization is computed separately from needs. | Add a second graph over existing groups, keeping runtime needs separate. |
| `crates/ilang-check/src/types/infer.rs:155–191`, `:233–355` | External types are converted first; groups allocate holders, infer/link their values, generalize, then finish known types and coverage. | Complete value-type reads at explicit group/module events, not in a lazy alias conversion. |
| `infer.rs:42–56`, `:1000–1005`, `:2052–2100` | A scheme has a level; mono schemes use `u32::MAX`; instantiation copies only quantified variables; local bindings are checked in order. | Keep the same instantiation and sharing rules for annotations. |
| `infer.rs:852–870`, `:953–970`; `types/known.rs:102–137` | `conv` and `kconv` expand aliases lazily, with depth fallbacks to a fresh variable and `Top`. | Value-dependent definitions need a completed cache; a depth fallback must never invent their meaning. |
| `infer.rs:2108–2120`; `types/display.rs:515–555` | Generalization preserves weak variables; mono display follows lower bounds **and** reverse `from` links. | Display is not evidence that a weak variable is immutable; a closure walk must include reverse links. |
| `types/solve.rs:245–285`, `:326–345`, `:385–447`, `:452–507` | Constraints propagate both ways through bound lists; environment arguments compare equally; same-side bounds can create more constraints. | Covariant occurrence alone cannot prove a read stable. Keep existing propagation when fixing a variable. |
| `types/solve.rs:153–170`, `:577–608`, `:618–633` | Extrusion and freshening also write bound lists directly. | A check only in `add_bound` cannot enforce the earlier draft's replacement invariant. |
| `infer.rs:569–597`, `:983–989`; `resolve.rs:2394–2486` | Written exported annotations are checked in resolve and skipped by the inferred export check. | Value-dependent annotations must be deferred to the inferred check instead. |
| `resolve.rs:797–803`; `crates/ilang-check/src/lib.rs:35` | `Alias(..)` is accepted as `Alias`. | Pattern binders retain that behavior; they do not gain constructors. |
| `crates/ilang-syntax/src/ast.rs:327–338`; `types/pattern.rs:157–185` | A typed pattern binds a name/discard; a whole tuple is not followed by a type annotation. | Use individually annotated tuple parts in examples; no new pattern grammar. |
| `crates/ilang-check/src/query.rs:26–48`, `:342–346`, `:492–545`, `:991–1053` | Queries infer after errors, use HIR type spans for targets, and collect symbols within item spans. | Failed reads need explicit recovery; give each binder its own span and preserve its whole source item. |
| `crates/ilang-cli/src/commands/package.rs:1063–1104`, `:1235–1249`, `:1306–1323` | Value interfaces use inferred types; alias interfaces print written bodies; canonical names are separate from display names. | Serialize resolved alias bodies, not a private source's spelling. |
| `crates/ilang-check/src/hir.rs:167–192`, `:273–296`, `:349–357`; `types/mod.rs:148–199` | Aliases have ordinary TypeIds; value references distinguish local/def/constructor; Typed has no alias-body table. | Reuse TypeDef/ValueRef and add a result table. |
| `crates/ilang-wasm/src/lower.rs:2209–2237` | Type-test lowering expects real heads; an unexpanded alias can fall back to an impossible tag. | Assert that extraction aliases never survive into runtime type-test heads. |

## 3. Alternatives and choice

Criteria: the same set-of-values explanation in every use; principal inference under explicit rules; the schema example; no order- or consumer-dependent meaning; import-stable parsing; a practical implementation in the checked compiler.

| Design | Advantages | Failure or cost |
|---|---|---|
| A. Read §4.8's known type everywhere. | Simple shape information; available early for some locals. | A fitted literal may have known type `"a"` but inferred type `String`. Other names have no known type. It does not mean the type of the value's scheme. |
| B. Instantiate/share the live scheme everywhere, including aliases. | Closest to expression lookup; preserves ordinary constraints. | A permanent alias may contain a weak variable changed by later uses, or instantiate an identity function independently at incompatible uses. It is not a fully determined structural alias. |
| C. Only read already closed or generalized schemes; reject every remaining weak variable. | Small and sound; no new commitment rule. | Rejects records.md §5.9's non-generalized builder. Requiring the user to rewrite its full record type defeats that accepted example. |
| D. Give top-level read parts an explicit completion point; close them there, using checked equalities. | Stable aliases and imports; supports partial codec extraction; uses the ordinary solver. | Reading can restrict later uses; ambiguous/unfilled parts are errors. Requires scheduling and a careful closure walk. |

Choose **D**, with B's ordinary annotation instantiation and local sharing, C's rejection of unresolved parts, and A's early *shape* information for locals, without substituting their narrowed known types for their inferred types.

Fixation is a language rule, equivalent to adding a determined type annotation at the source's completion point. It is not a claim that a lower-bound snapshot is the most general unconstrained solution. Once these specified equations are added, inference still computes the principal solution of the constraints; it does not search for a solution to make a pattern match. The spec must describe this commitment explicitly (§7.5). A rule that silently adds bounds while merely printing a type is excluded.

## 4. Exact rules

### 4.1 Names, sites and scope (V)

**V1.** A lowercase `x`, or `m.x` with lowercase final name, in a type and without following `(`, means a **value type**. `{ x }` groups it. `{ x: T }` is a record type after T3. With following `(`, `x(…)` and `m.x(…)` are TypeCalls reserved for macros. Imports never change that classification.

**V2.** Resolve `x` as an expression name at that place: locals, the module's own/imported value scope, then built-ins. Built-ins are ordinary prelude definitions (HIR `ValueRef`, `hir.rs:349–355`). A module used alone gives E0205; an unknown name gives E0201. `m.x` is only a module member, with the normal export check. A value named `m` gives E0207; no field projection is available in a type. This does not inspect `x`'s runtime value.

**V3.** Before resolving the types of a pattern or function header, collect its pending value binders. In those types, a name of that same pattern or parameter list is E0536, even if it would otherwise find an outer name. The function header includes `->`, `?`, and `!`. Inside a binding's own value, a type may not refer to that binding's own value names, at any nesting depth; ordinary recursive *expression* uses remain legal. Remove the function-header pending names on entry to the body, where parameters become available. For the qualifier of `m.x`, a pending value name gives E0207.

Thus `(x, y: x)`, `(x) -> x { x }`, and `f = (g: f) { g }` are errors. In `f = (x: Int) { y: x = x  y }`, the reference to `x` is allowed. Local bindings and clause patterns follow the same rule. Use two scope mechanisms: a current-pattern/header pending set, checked before outer lookup, and an own-binding reserved frame installed for its RHS. Inner binders that have entered scope cover that reserved frame. Expression lookup ignores reserved frames; type lookup reports E0536 on them. Thus an inner parameter named f may be read in its body even inside the outer binding f; a non-direct local RHS cannot silently capture an outer f as its own type. Track binder identities and lexical depth, not a global set of forbidden text.

**V4.** A value type names `ValueRef::Local` or `ValueRef::Def`, never a constructor. Definitions are top-level only. T3 reuses these resolved references to reject a binding with label parameters (E0542, records L5) or a key parameter (E0543, L2a), including pattern sources. This agrees with records.md question 10.

### 4.2 What is read (R)

**R1. Annotations.** For a completed generalized binding, take a fresh instance of its scheme, exactly as for an expression use (§4.8 rule 3). One quantified variable becomes one fresh variable per occurrence of the value type, shared within that occurrence. Separate occurrences do not share their instantiations. This applies to generalized locals, top-level names, imports and built-ins alike. The resulting annotation does not make its own binder polymorphic.

For a parameter, clause binder or non-generalized local in the current top-level group, share its existing monotype. This includes weak variables captured by a local scheme: they remain shared, as they do in an expression use. They are not copied or completed by a local annotation. A completed local scheme's quantified variables alone are freshened.

For a completed top-level definition, the read parts of its scheme use the fixed forms of R3–R5 below. Quantified variables are still instantiated, not fixed. A weak part not fixed by its source module is E0534. Reading through a local does not turn an unfixed *top-level* read into an allowed fresh copy: shared weak variables stay shared; no structural alias can escape from such a local.

**R2. Definitions.** In an alias body, every `Of(x)` must give a closed type: no quantified variable, unresolved weak variable, label parameter, `Top` or hole. This applies to `T = x` and to occurrences deeper in `T(A) = (A, x)`. The alias's own formal parameters remain formal parameters; the closure requirement concerns the value-derived part.

For an alias pattern, only a binder's part, an exact part introduced by expanding a head alias, and the shapes needed to reach them are demanded. A skip demands no content. Quantified variables under a skip are fine. A used quantified variable gives E0533. There is no “pick an instance that matches” operation.

**R3. Completion point and ownership.** Collect demands before inference, including unused aliases/patterns. Only a demand *written in the source definition's own module* can fix that source's weak parts. Transitive uses of an imported alias do not count as new demands in its source module. Imported readers never fix another module. They may read a closed structural part or a part already fixed there; otherwise E0534 at the reader.

At the end of a source's ordinary expression group, after linking and generalization, take a single snapshot for **all** its module's demands. Record the first solver-variable index before allocating the group's holders. All demands in that group see that snapshot, so one pattern cannot supply information to another. An earlier successfully fixed variable has its recorded closed form.

A group that constrains any earlier, unfixed weak variable is **open**: no weak variable of that group can be fixed. Detect this throughout constraint propagation, including `compare`, `reach`, extrusion and reverse links, not just at the initial operands. Also reject a demanded part that merely carries an earlier unfixed weak variable, even when no constraint touched it. “Earlier” uses allocation ownership, not the current level. Quantification is relative to the source's Scheme.level, not a global “general” flag. These conservative checks prevent unrelated earlier groups from changing a snapshot.

**R4. Closed forms.** For a demanded weak variable of this group:

1. Follow its lower-variable edges **and reverse `from` edges**, and collect every lower bound. Pure variable SCCs are collapsed; they are not recursive types. The checked display walk includes both kinds of edge (`display.rs:531–538`).
2. Replace each weak variable needed inside a bound by its closed form, regardless of polarity. Earlier fixed variables use their recorded forms. A quantified variable needed here, an earlier unfixed weak variable, `Top`, an unsupported form or a structural recursive dependency makes the part unavailable.
3. Keep every bound, including unions, intersections, differences and function types. Normalize only with exact type equalities. There is no “ignore an unfamiliar bound” arm. T1 value sets are kept exactly. T3 adds record/Without forms with its exact rewrites.
4. The candidate is the union of those closed bounds. No concrete lower bound means E0534; do not invent `Never` or take an intersection of upper bounds. An explicit closed `Never` is allowed. A pure-variable SCC with no concrete lower bound is unfilled.
5. Stage equalities `v <: candidate` and `candidate <: v` for every variable whose form was needed. Check the **whole batch** with the ordinary solver, retaining all original bounds, caches and reverse links. Also stage the structural-shape checks of P4. Check occurs and places. Commit only if the batch succeeds and touches no earlier unfixed weak variable. Otherwise give E0534 and discard the trial state. Never assert that these equalities must succeed.

Candidate collection reads an immutable snapshot. Closed-form normalization and exact-part subtype comparisons must use pure closed-type operations or an isolated scratch solver; they may not add source bounds or feed trial-derived information back into candidate selection. Every normalization result is checked to remain closed.

A solver checkpoint/trial copy is an implementation choice; its rollback includes variables, levels, bounds, reverse links, constraint/comparison caches, touched/reported sets and failures. No trial failures leak into the main diagnostic stream. This path runs only for demanded weak parts, not for every binding. Adding Solver cloning for a first implementation is acceptable; measure before adding an undo log.

**R5. Fixing means constraining.** Keep the equations in the ordinary solver and record `fixed[v] = (closed_type, reading_site)` for extraction, display and diagnostic notes. Do **not** replace a variable at `constrain_in`, erase its old bounds, bypass `reach`, or forbid later bound insertion. Existing relationships must keep propagating. A later incompatible use gives the ordinary type error with “this type was fixed here” at the reading site. Extending a source annotation to a wider type is the remedy.

Fixation never generalizes a weak variable. Variables not demanded remain shared and unfixed. A foreign top-level type read adds no fixation equations to its source. Adding only such reads cannot change the library's completion results. Ordinary expression uses and local monotype sharing can still constrain shared weak variables, as §4.7 already permits; this rule does not introduce separate compilation or make every shown weak scheme consumer-independent.

**R6. Examples and cost.** With `id = (x) { x }`, `twice = (f: id, n: Int) { f(f(n)) }` instantiates `id`; `Same = id` is E0533. `g = id(id)` is weak and unfilled, so `G = g` is E0534. With `port = 8080`, `Port = port` is `8080`, never `Int`; write `port: Int = 8080` to obtain `Int`.

With an invariant `Cell(A) { Cell(get: () -> A, set: (A) -> Nil) }`, `r = cell(1)` and `C = r` in one module fix the demanded parameter to `1` if the batch checks. A later `r.set(2)` fails. Without the read, later constraints may widen that parameter. This is a deliberate, visible cost of D; it is not ordinary display simplification.

### 4.3 Alias patterns (P)

**P1. Classification is syntactic.** An item with an UpName head and `=` has an alias-pattern head exactly when it has arguments and its *written* right side, after stripping type groups, is a value type. No arguments gives a plain alias `T = x`. Every other definition head is a list of type parameters, checked by syntax.md §4.3.

Thus `Pair(A) = (A, A)` is generic; `Schema(User) = user` is a pattern even before importing anything. An import can make a binder conflict, or a head unknown, but cannot change which item it is. `Schema(User, _) = make_type()` remains a generic-alias form with an invalid parameter `_`; expanding its right side cannot turn it into a pattern.

Both parsers store this distinction in the core tree **before expanding children**. An item macro may create a new alias-pattern item, classified when its output becomes an item. A macro cannot change an existing generic alias into a pattern by expanding its body. An `AliasPattern` node has a head reference and its binder list; `Definitions.definition(User)` returns that written item without needing its inferred type.

**P2. Heads.** The outer head is an unqualified type reference, not a definition. `m.Schema(User, _) = user` starts with a lowercase name and is an ordinary constructor-pattern binding under accepted §2.2. On a failing such binding with a lone value RHS, suggest importing the type and writing the unqualified head.

A head is a custom/built-in type with the right arity, or a transparent alias that expands to one named type or tuple. A head alias must not depend on a value type, contain a hole or have any free variable other than its declared parameters. Each formal parameter occurs at most once, only through named arguments or tuple parts. Replace formal parameters with the corresponding pattern parts. A parameter below a function, union, intersection or difference is E0532. Constant subtrees become exact checks: the source part must be closed and equal in both subtype directions. This supports renaming and tuple projection:

```i
Pair(A, B) = (A, B)
Pair(Tree, _) = parsed
```

There is no new tuple-item grammar. `(Tree, _) = parsed` is still a value binding. Recursive aliases are E0223, not a depth-limit fallback.

**P3. Parts.** A bare unqualified UpName binds a fresh zero-parameter structural alias, even if that text already names a type. A duplicate module definition is E0208; a collision with an imported/built-in type is E0537. It never silently becomes an exact type test.

`_` is a skip node, never a type hole. `m.T`, `T(P, …)`, `m.T(P, …)`, and tuple patterns recursively match shapes. A qualified zero-argument type is therefore an exact nominal-head test. Nested aliases follow P2. Records, functions, literals, unions, intersections, differences, value types and macro calls are not pattern forms in T2 (E0532). A record may still be the closed type bound by `User`; T3 adds no record-pattern syntax in this note.

**P4. Matching is observation, then checked completion.** From the source scheme, expose the demanded outer structure without choosing a type for a quantified variable. A weak variable's lower-variable SCC may expose its normalized lower bounds; an unfilled variable gives E0534. At each named/tuple node require exactly one normalized member with that head/arity. Do not discard extra union members or select a convenient alternative. Mismatch is E0535.

Record every weak variable traversed to expose a shape. If it is not already fixed, it must be owned by this group and not open (R3). In the trial batch, require it to be a subtype of the observed skeleton, **reusing its original argument variables**, and recheck that skeleton after the batch. This closes the observed outer shape without closing a skipped argument. No fresh “hole” is made for a skip. An ambiguous choice needed to expose a skeleton, or an unknown head, fails; no search is permitted. Do not inspect skipped contents for matching or impose a new written-type place check inside them. The source's ordinary checks still apply. Do not stage a fresh argument to replace an existing weak variable, even when that argument occurs only under a skip.

At a binder, compute a closed form by R2–R4 and store it as the alias body. At an exact subtree, close first and compare with the already closed expected type; the expected type cannot fill in missing information. At a skip, do nothing to that argument. A skip's variable may nevertheless be fixed if the same solver variable is also demanded elsewhere; skipping never breaks existing sharing.

Example: a source with type `Schema(D, E)` and closed decode candidate `Int` gives `Schema(Size, _) = size` the alias `Size = Int`. It leaves the distinct `E` variable live. A source `Schema(A, A)` cannot promise that its second argument stays unfixed: both arguments are the same variable.

**P5. Completion is eager, dependency ordered.** Check every pattern, even one with no binders, and every value-dependent alias, even unused. A site becomes ready after all its source groups and dependent alias sites complete. Process dependencies before dependents; use source order only to break ties. This matters for `A = List(B)` written before a pattern defining `B` from the same source.

At module start, complete sites whose sources are declarations, built-ins or completed imported modules. Do this also for a module with **no value bindings**. After each group, complete newly ready sites. Store Pending / Ready / Failed; each successful definition is read once. All sites use the pre-batch snapshot of their source's group (R3), not the solver as changed by whichever alias happened to be used first.

### 4.4 Order and cycles (O)

**O1. Resolution before inference.** Phase 1 collects DefIds and pattern binder TypeIds. Phase 3 resolves value types in definitions using the already collected/imported value scope; it never asks for a type. Phase 4 resolves annotation value references and locals. After the alias graph is checked, compute dependency summaries for aliases/pattern binders and walk **all types** in every binding, including inner bindings, parameter/clause patterns, type tests and return/effect types. Keep a single reusable HIR visitor.

Dependency summaries include the aliases named in pattern heads/parts as well as their sources. They include declarations/imported definitions for readiness even though these need no same-module binding edge. A memoized traversal avoids repeated expansion; materializing transitive sets costs at least their output size, not necessarily linear time.

**O2. Two graphs.** Compute ordinary expression-use SCCs exactly as before. Over those groups, add ordinary dependency edges and type-read edges: the reader depends on every same-module source group reached through its annotations/aliases. A type edge within one SCC, a self edge, or a cycle in this second graph gives E0530. Never merge groups on type edges, since that could change generalization.

Runtime needs, full needs and initialization continue to use only expression uses. A type lookup can require compile-time completion without requiring runtime initialization. Imported modules complete first under the existing module DAG.

**O3. Diagnostics.** Keep edge provenance: annotation span, alias chain, source binding and expression-use path. For:

```i
a = (n: b) { n }
b = a(1)
```

report E0530 at `b` in the annotation: “the type of b is needed to type a, but b uses a; write the type instead.” For `A = List(A)`, give E0223; for imports in a circle, retain E0214. Do not turn any of these into “type not known”, a fresh variable or a depth-limit result.

**O4. Recovery.** Mark definition sites, pattern binders and annotation sites in a bad dependency component Failed before inference. Dependents propagate failure without another diagnostic. Normal checking stops after resolution/order errors (`lib.rs:160–169`), but query checking continues (`query.rs:36–41`). Pass an explicit prior-error flag to the checker; don't infer it from an empty inference diagnostic vector. In recovery only, a missing dependency gives Failed/Error and preserves unrelated query results. A Pending read in an otherwise valid program is an internal scheduling error.

### 4.5 Known types, places and forbidden contexts (K)

**K1.** `Of` reads inferred schemes/monotypes, never a narrower flow fact. For a completed generalized scheme, its known form erases quantified variables to `_`, keeping relationships only in inference. A completed fixed part has that closed known form.

For a current parameter or monomorphic local, kconv uses a pure **inferred-shape conversion** of its stored monotype: substitute already fixed closed forms, retain its structural nodes, and erase every remaining inference variable to a hole, without following its temporary bounds. A written type on that name is already present in this monotype (`types/pattern.rs:178–185`). For completed generalized locals use the simplified principal scheme while preserving weak variables, then erase remaining variables; do not substitute a value-known fact for a weak variable.

No separate flow-shape table is needed. Never use fit_known's kvars, expected known types, or the known type of a value matched by a compound pattern as the inferred type read by Of. They can be narrower (`known.rs:457–466`; T1 §4.8). An unannotated source parameter stays unknown here even when its callback has a narrower expected known type. Ordinary expression checking still uses expected types as before.

```i
copy = (s: String) {
  x = "a" = s
  y: x = "b"
  y
}
```

With `"a"` as input this gives `"b"`; the fitted binding needs Assert in its environment. The inferred type of x is String, its value-known type is "a", and y uses String. The inferred-shape conversion must not copy that narrower fitted-literal fact. Likewise, `((a: Int, b: String)) { q: a = a  q == 1 }` uses `Int`, not the whole tuple. Conversion to known information never shares or commits new inference constraints.

**K2. Places.** Check the **whole expanded annotation**, including its surrounding union/intersection, with value types represented by their inferred shape before quantified variables are freshly instantiated. Treat unfilled shared local holders as type variables. Checking each `Of` independently misses `a | b` when both are variable members. Reuse §4.4's polarity and T3's spread restrictions; no new choice in input places. A `pick(A, B)` of type `(A, B, Bool) -> A | B` cannot be smuggled into a parameter annotation through `(f: pick)` (E0518).

Keep T3's check on shown types of non-generalized bindings as well. Alias extraction and fixing never waive that accepted check. Type tests retain §7.1's ban on a variable member; T1's literal test members keep their value sets.

**K3. Early/trusted contexts.** Directly or through an alias, a value type is E0531 in a custom constructor field, an external declaration, a host declaration/type, or a service/environment type, including `!` components, environment parameters and statements. Check this before early conversion, reporting E0531 without an additional shape/ABI error. It also rules out embedding an extraction alias in such a field. This is an ordering limit, not a special rule for a library.

Ordinary annotation parameter/result/error types and type tests are allowed. Reading an ordinary function scheme that has an environment is allowed; writing a new value type *inside* an environment type is not. The inferred environment continues to obey ordinary service equality. Constructor fields stay complete before inference, as the macro draft's step 1 promises.

### 4.6 Exports, queries and package interfaces (X)

**X1.** Pattern binders are ordinary alias TypeDefs with their binder spans and no constructors/parameters. `(User)` exports the alias; `(User(..))` has the same effect. Neither exports constructors of a nominal type that the alias happens to name. The source value need not be exported.

**X2.** After completion check exported extraction aliases, all exported aliases transitively using them, and exported top annotations transitively using value types against §2.4. Private nominal types of the exporting module may not leak. Written types with no value dependency retain the existing resolve check. With a value dependency, defer the annotation's entire hidden-type check; remove it from the “already checked written annotation” exemption in inference. Expand private aliases and retain source provenance in messages.

**X3.** Add `Typed.alias_types: Map<TypeId, ShownType>` for each extraction binder and value-dependent alias, parameterized by its own written formal parameters. Package interfaces compare the completed structural body, with canonical nominal identities, complete literal sets and canonical formal-parameter identities. Never serialize `= private_value`, solver variable ids, source locations, fixation metadata, or shortened hover text. A change to a private source can change its exported alias. A source rename with the same read type does not. T3's label canonicalization remains in force.

**X4.** Query type printing shows the written value name in annotations. Hover on that reference shows the source's scheme; go-to-definition uses its resolved local/DefId. A pattern head targets its existing type; each binder targets its own TypeId. Hover on a completed binder shows `User = <structural type>`; on a failed/pending binder show its written pattern and available diagnostic, never an invented hole alias. The outline lists every binder even when the item has several. Resolve source references even when inference failed. Completion/type navigation uses completed alias bodies when available.

### 4.7 Records and macros (M)

**M1.** The records.md §5.9 builder gives a weak `Codec(D, E)`. Demand only its decode part and the Codec shape. Normalize `Without(R, {age}) & { age: Int }` after closing its dependencies; T3's exact rewrites yield `{ age: Int, name: String }`. Fix that decode part through checked equations. Leave the distinct encode variable unfixed; `user.encode(u)` still constrains it normally. Reading `Codec(_, Enc) = user` with no encode lower bounds is E0534. No case for “schema” or “codec” exists in the algorithm.

T3 must add closure, provenance and inferred-shape arms for records/Without, rerun its L2a/L5 checks on `Of`, and test this builder. T2 itself tests an ordinary nominal Codec with closed scalar decode output. Until T3 adds those forms, unsupported closure is an explicit failure, never a partial type.

**M2.** Syntax-macro output is resolved and ordered like written code. Hygiene marks apply to value references, head type references and binder definitions. Classification is stored as P1 says; pattern parts are not type positions and cannot be macro calls. `Definitions` returns written items, not `Typed.alias_types`. A type definition is available for syntactic inspection before its inferred alias body is ready.

The Lisp reader/printer needs rows for value type `x` / `(. m x)`, and pattern item `(= (Schema User _) user)`, with a distinct pattern-part position. Add these in whichever of M1/T2 lands second. Groups do not change classification. This agrees with the draft's core-tree/group rule (§1).

**M3. Typed macro boundary, for M4.** Expansion happens after ordering. A newly generated direct value type gives E0531, including a local one in this first version. A generated reference to an existing value-dependent alias is allowed only when its completed definition is guaranteed available: every same-module source is in a strict ancestor group of the call, and every imported source's module has completed. A coincidentally earlier group in one tie-break order is insufficient. A module-start alias with no binding sources is available. Apply the same rule to type syntax read by `Types`; no “not ready yet” depending on scheduling.

Never add demands or fixation equations during typed expansion: M4's “no new bounds on caller variables” rule remains intact. M4 reuses T2's readiness/context checks and T3's label checks. These are additional, explicit answers to meta.md's T2 question; that draft must be updated before M4 lands.

## 5. Errors

T2 proposes E0530–E0537, leaving E0523–E0529 to T1 and E0540–E0549 to T3. Confirm the allocation at integration; only T3's block is already reserved by its accepted note.

| Code | Meaning |
|---|---|
| E0530 VALUE_TYPE_CIRCLE | Type dependency inside a value SCC, or cycle between groups. Include a chain. |
| E0531 VALUE_TYPE_PLACE | Forbidden early/trusted context, or unsafe typed-expansion read. |
| E0532 TYPE_PATTERN_FORM | Invalid pattern, macro part, or non-projectable head alias. |
| E0533 VALUE_TYPE_OPEN | Used quantified variable, hole, Top or label parameter in a definition read. |
| E0534 VALUE_TYPE_NOT_FIXED | Unfilled/recursive/unsupported weak closure, open group, foreign unfixed part, or failed completion equations. Say which source part and suggest its annotation. |
| E0535 TYPE_PATTERN_MISMATCH | Wrong head/arity/union, or closed exact part not equal. |
| E0536 VALUE_TYPE_OWN_SCOPE | Same pattern/header or own binding value. Explain an outer name that would otherwise be captured. |
| E0537 TYPE_PATTERN_BINDER_IS_TYPE | A new pattern binder conflicts with an imported/built-in type. |

Reuse E0201/E0203–E0205/E0207/E0208, E0221–E0224, E0227, E0501/E0503/E0515/E0518 and T3's E0542/E0543. A later failure caused by fixation retains its ordinary mismatch/invariant/environment code plus the fixation note. A3's temporary lowercase-type error and A1's “reserved alias pattern” error are removed with the feature, not reassigned to these cases.

## 6. Why the rules meet the language requirements

A value type is a way to write a set of values already described by a scheme/monotype. An alias pattern projects that structural description; it never examines runtime data or invents a nominal identity. Schemas, parsers and callback wiring exercise unrelated uses.

Annotation instantiation, monomorphic sharing and the value restriction are unchanged. A definition read cannot create polymorphism. Fixation adds deterministic closed equations at a stated boundary; matching never constrains an unknown to make it fit. All constraints then use the same subtype solver and place restrictions. This is the principal-inference argument for the proposed language rule, not a machine-checked proof.

A closed alias cannot change after being published. Equality constraints retain all source relationships, so a hidden setter or service use must still agree with it. Module ownership and rejection of earlier open variables make the completion and its failure independent of unrelated groups and consumers' new fixation demands. Existing expression constraints and local monotype sharing remain shared; the claim does not cover the display of uncompleted weak parts. Type edges cannot enlarge a recursive group or change runtime initialization.

## 7. Replacement spec text

Apply after syntax.md and T1's replacements; T3 subsequently adds record syntax/places. Keep existing section numbers. The following is normative proposed text; implementation names above are not part of it.

### 7.1 §2.4 Exports: add bullets

> - A type defined by an alias pattern (§4.3) is exported as an alias. It has no constructors of its own; exporting it with `(..)` has the same effect as exporting it without `(..)`.
> - An exported alias or top annotation that uses the type of a value, directly or through aliases, is checked after that type is found (§4.7). It must obey the same rule about private types. Its source value need not be exported. Changing a private value can therefore change an exported structural alias.
> - Reading a value's type from another module never fixes a part of that value's type (§4.7). Adding a consumer or a test module cannot change the types fixed by its source module.

### 7.2 §2.5 Top-level bindings: add after Order

> **Type dependencies.** A type may name a value (§3). This does not work out that value or add it to runtime needs or full needs. It only orders type inference: a binding whose annotations use the type of a value of its module, directly or through aliases, is typed after that value's group (§4.7). A type dependency within one group, including on itself, or a cycle between groups is a compile error. The message gives the chain of dependencies.

### 7.3 §3 Type syntax: grammar and bullets

Keep `TypeCall` and its reservation from syntax.md, and literals from T1. Name the existing lowercase alternative:

```
Prim      = FnType | TupleType | NamedType | TypeCall | ValueType
          | Literal | "_" | "{" Type "}" .
ValueType = [ name "." ] name .
```

Replace the temporary lowercase-type error bullet:

> - A lowercase `x` in a type, without `(` after it, means the type of the value named `x` at that place. The name follows value scope (§5). `m.x` means an exported value of module `m`, never a field of a value. If `m` names a value, it is a compile error. `{ x }` groups the same type. A lowercase name or member followed by `(` remains a macro call, not a value type or a type application.
> - A type may not name a value bound by that same pattern; a parameter in the types of that function's own header, including its `->`, `?` and `!` parts; or its own binding's reserved name inside that binding's value. A distinct inner binder that has entered scope may cover that reserved name. This is a compile error even if an outer value has the same name. Those parameters may be named in types in the body. For example, `(x, y: x)` and `(x) -> x { x }` are errors, but `(x: Int) { y: x = x  y }` is allowed.
> - A value type is not its known type (§4.8). It follows the scheme, instantiation and completion rules of §4.7. It never makes a monomorphic value polymorphic.
> - A value type, directly or through aliases, may not occur in a custom constructor field, an external or host declaration, or a service/environment type. Those types must be complete before inference.
> - A type variable is an UpName declared after a binding, declaration, or type-definition name (§4.7). Lowercase names do not declare type variables.

### 7.4 §4.3 Custom types: replace with this grammar and bullets

```
TypeDef     = UpName [ TypeHead ] TypeBody .
TypeHead    = "(" Type { "," Type } [ "," ] ")" .
TypeBody    = "{" { Constructor } "}" | "=" Type .
Constructor = UpName [ "(" Field { "," Field } [ "," ] ")" ] .
Field       = [ name ":" ] Type .
TypePattern = UpName | "_"
            | [ name "." ] UpName "(" TypePattern { "," TypePattern } [ "," ] ")"
            | name "." UpName
            | "(" [ TypePattern { "," TypePattern } [ "," ] ] ")" .
```

> - `T { C1 C2 … }` defines a nominal custom type. Each constructor has zero or more fields, which may have labels. Two custom type definitions are different types even if their fields are the same.
> - `T {}` has no constructors; i code cannot make its values. They come from the host (§10).
> - `T = Type` is a transparent alias. Aliases may not use themselves, directly or through other aliases or alias patterns.
> - An item `T(P1, …, Pn) = v` is an **alias pattern** exactly when its written right side is a value type (§3), after removing groups. Otherwise its head is a list of type parameters. This classification is made before expanding the right side; it never depends on imports or on what a macro returns there.
> - In a list of type parameters, each part is a different, unqualified UpName with no arguments, and must not already name an accessible type. Every parameter must be used in the body, and every type variable in the body must be a declared parameter.
> - In an alias pattern, the unqualified head names an existing type, with the written arity; the item does not define that head. It may be a custom or built-in type, or a transparent alias expanding to one named type or tuple. In a head alias each parameter is used at most once, only as a named argument or tuple part. Constant subtrees of that alias require an equal, fully determined source part.
> - A bare unqualified UpName in a pattern always defines a new zero-parameter structural alias. Its part of the source type must be fully determined (§4.7). It may not duplicate a definition or conflict with an imported or built-in type. It never tests an existing type with that name.
> - `_` in an alias pattern skips a part and binds nothing; it is not an inference hole. Elsewhere in a type, `_` remains a hole. Shared type variables remain shared even if one occurrence is skipped.
> - A named pattern with arguments, or a qualified named pattern, requires exactly one member with that type constructor and matches its arguments. A tuple pattern requires one tuple of the same size. No union member is discarded to make a match. A mismatch is a compile error. Function, record, literal, union, intersection, difference, value-type and macro-call patterns are compile errors.
> - Every pattern and every value-dependent alias is checked, also when unused, after its sources and alias dependencies are complete. A definition read may not choose an instance of a generalized variable to make its pattern fit.
> - Blocks have no type definitions. An item beginning with `(` or `{` is a value binding; `m.T(…) = v` is also a value binding under §2.2. Import `T` to use it as an alias-pattern head.

```i
Codec(D, E) { Codec(decode: (String) -> D, encode: (E) -> String) }
make(D, E) = (d: (String) -> D, e: (E) -> String) {
  Codec(decode: d, encode: e)
}
size = make((s) { string.length(s) }, (n: Int) { int.to_string(n) })
Codec(Size, _) = size
// Size = Int; the distinct encode part is not fixed.

Pair(A, B) = (A, B)
Pair(Tree, _) = parsed
// If parsed has the closed type (Ast, List(Warning)), Tree = Ast.
```

### 7.5 §4.7 Type inference: add after groups, and refine variable declarations

> **Reading types of values.**
>
> - In an annotation, a completed generalized value is read as a fresh instance of its scheme (§4.8 rule 3), separately for each occurrence. A parameter, clause name or local monomorphic value shares its one type. Generalized local schemes also keep their non-general variables shared. A value type never generalizes its source or its annotated binder.
> - In an alias body or alias pattern, every used value-derived part must be closed: it has no generalized variable, unresolved type variable or hole. The alias's own declared formal parameters remain parameters. A skip demands no contents, but the shapes needed to reach the used parts must also be determined.
> - A top-level value's group has one **completion point**, after its values are typed and linked and its generalization is decided. All value-type demands written in its own module are collected before inference, even in unused type definitions. At this point their weak parts are fixed as follows. A demand in another module never fixes them.
> - For a demanded non-general variable, collect all types that come to it, including through variable relationships. Put them together by union, replacing the non-general variables needed inside them by their fixed types. Pure cycles of variable relationships count once. A structural recursive type, an unfilled part, a generalized variable needed inside this replacement, or a part that cannot be made closed is a compile error. An explicit closed `Never` is allowed; the absence of any information does not invent `Never`. Literal types are never widened.
> - Fixing adds both subtype constraints between the variable and the resulting closed type, keeping every previous constraint. Every needed variable is fixed together, and the usual occurs and place rules apply. If these equations cannot hold, it is a compile error. Pattern shape checks use the original argument variables, so skipping a distinct argument leaves it shared and unfixed. The pattern cannot supply a type to fill an unknown.
> - A group that constrains an earlier non-general variable that was not fixed cannot fix its own non-general parts. A demanded part carrying such an earlier variable also cannot be fixed. This conservative rule makes the result independent of unrelated groups' inference order.
> - From then on, later uses must agree with each fixed part. A use that would widen it incompatibly is a type error with a note at the place that fixed it. Write a wider annotation at the source when needed. For example, reading `r = cell(1)` may fix its invariant argument to `1`; write `r: Cell(Int) = cell(1)` to allow other Int values.
> - In another module, a read may use the source's generalized variables by instantiation, or its closed/fixed parts. A demanded weak part that the source module did not fix is a compile error at that read. Adding foreign fixation demands cannot change the source module's completion result. Ordinary expression constraints and local monomorphic sharing remain shared as elsewhere in this section; this rule does not fix the display of every uncompleted weak part.
> - A permanent alias denotes the completed closed part. It does not change when later bindings are checked. Definitions are completed before their dependents, even if written later in the source.
> - The whole annotation after value types and aliases are read must obey §4.4's rules of places. A value-type spelling does not hide variable members from that check.

Replace syntax.md's declaration bullet ending “after the name of a type definition” with:

> - A type variable is declared after a binding name, a declaration name, or in the list of type parameters of a type definition. The parts of an alias pattern are not declarations of type variables: its bare UpNames define structural aliases.

Keep the principal-type and value-restriction bullets. Completion equations above are explicit typing constraints, not an operation of type display. Do not describe this as merely substituting a shown string.

### 7.6 §4.8 Known types and pattern types: edits

Replace the pattern-type bullet for `x: T`:

> - `x: T`: `T`, with new variables for holes and value types read as §4.7 says; `x` gets that type.

Add under Names in patterns:

> A value type in an annotation uses the inferred type of its source, not a narrower known type at the use. A completed scheme gives its known structural form, with `_` for general variables. For a local source still being inferred, use its stored structural monotype with unresolved variables erased to `_`. A written type on the source name is part of that monotype. The known type of a matched value does not supply a type for a name inside its pattern. Do not infer a smaller shape from temporary lower bounds. Later literal tests or clause narrowing do not refine this shape. Thus `((a: Int, b: String)) { q: a = a  q == 1 }` gives `q` the known type `Int`. For `x = "a" = s` with `s: String`, x's inferred type is String even though its known type is "a"; an annotation naming x uses String.

### 7.7 §5 and future macro text

Add to the type-variable scope bullet: “An alias pattern has no type parameters; its binders are types of the module.”

When M4 adds typed-macro rules, insert M3 verbatim in its spec section with language-level dependency terminology. T2 does not add an implemented macro facility or change the reservation of TypeCall in §3.

## 8. Representation and files to change

Paths below are proposed changes, not changes made by this note.

- `crates/ilang-syntax/src/ast.rs`, parser/type-head region and formatter/printer arms: retain A3's value-type node; add the P1 core-tree distinction and binder/part structure. Validate pattern forms after classification, not by consulting imports. Keep group-insensitive classification and TypeCall. Remove temporary A1/A3 errors. Update `crates/ilang-check/tests/hir.rs` and syntax round-trip fixtures.
- `crates/ilang-check/src/hir.rs:56–79,167–192,273–296`: add `TypeKind::Of { value: ValueRef, site: OfId, projection: Option<ProjectionId> }`, site/context/provenance tables and the shared type visitor. Add AliasPattern and TypePat (Bind/Skip/Named/Tuple/Exact/Error). Each binder remains TypeBody::Alias(Of(...)), with no new TypeBody variant. Store its full item separately for queries/Definitions.
- `resolve.rs:391–592,773–803,1098–1209,1329–1379`: collect pattern binders before exports/imports; resolve definition sources from the value scope; add pattern alias edges, summaries and demands. Do not register the head as a new type. Check E0537 after imports. Collect an AliasPattern slot for items with multiple binders.
- `resolve.rs:1445–1660,1680–1755,1834–1905,2133–2202,2392–2486`: pending identities, value lookup and forbidden contexts; defer value-dependent hidden-type checks. Reuse one context/readiness check for M4 and one resolved-reference list for T3. All child/substitution/named-type walkers need Of arms.
- `order.rs:17–135,188–251`: keep the old use graph and initialization; add the group dependency graph and provenance. Store source completion events and alias-site dependency order, including modules with no groups. Add an alternate topological tie-break mode for tests.
- `types/infer.rs:72–149,155–228,233–355`: add demand snapshots, ownership/availability, completed read schemes, definition-site states, alias results, and a module loop around group processing. Completion must run after generalization and ordinary group checks; failed groups publish Failed, never partial aliases. Process module-start events even without groups.
- `types/infer.rs:603–687,852–970,1000–1005,1926–1992,2052–2120`: inferred-shape place checking, Of annotation instantiation/local sharing, closed definition cache, and pure inferred-shape conversion for locals. Replace the depth fallback for any alias chain involving value-dependent aliases with checked cache lookup; ordinary alias cycle recovery remains explicit.
- New `types/value_types.rs` or `types/alias_pattern.rs`: projection, closed-form collection including pure-variable SCCs/from edges, snapshot batch construction, P4 matching, failure provenance.
- `types/solve.rs:87–105,245–285,385–507,577–633`: trial/checkpoint support and optional group-origin observation for open-group checks. **No replacement-variable semantics and no bypass of bound propagation.** Existing constraints implement fixation. Include normalization operations in the observed trial state.
- `types/known.rs:102–162,433–515`: Of known conversion from stored inferred monotypes/schemes, never narrowed `kvars`; no solver-variable allocation in kconv.
- `types/mod.rs:148–210`, `types/display.rs:515–555`: alias results and exact fixed forms for extraction/display; preserve full typed value sets. Keep fixation metadata out of canonical types.
- `types/infer.rs:569–597,983–989`: deferred export checks and the written-type exemption. Check aliases even without exported value bindings.
- `query.rs:331–463,492–545,991–1053,1137–1147,1218–1249`: alias hover/fallback, binder outline, value-reference navigation and completed-body expansion. `query.rs:26–48` and checker entry plumbing pass prior-error recovery state.
- `crates/ilang-cli/src/commands/package.rs:1063–1198,1235–1385`: pass alias results to TypeShow and canonicalize completed bodies/formals. Human output uses UpName parameters; canonical identities may use internal numbered names. `TypeShow::param` currently prints lowercase (`:1281–1286`), so check A2's migration rather than assuming it was done.
- `crates/ilang-check/src/lib.rs:18–99`: diagnostic codes and prior-error state. `crates/ilang-wasm/src/lower.rs:2209–2237`: assertion/tests that alias TypeIds do not reach type-test heads; T1's literal test members remain intact.
- Spec sections in §7; syntax.md's temporary reservations become pointers to the implemented rules. Update records.md §11's code-allocation sentence and §19 question 10; meta.md's core-form/Lisp table, readiness rule and T2 question. These documentation edits belong to T2/M integration after this note is accepted; this lane changes only this note.

No-value-type programs bypass demand collection/closure/trial work. Do not promise literally zero allocations or an unchanged timing without measurement; existing code still receives new exhaustive-match arms and a disabled observation hook.

## 9. Ordered T2 checklist

1. Rebase on A1/A3 and T1; map the cited regions to the new code. Verify UpName declaration/display migration and the AST value/head forms. Implement any missing prerequisite within the approved T2 integration scope; report a scope conflict to the coordinator.
2. Confirm code allocation and cross-track edits; land §7 spec text first. Agree the fixation semantics, early-context limit and M4 boundary with G3.
3. Add core/HIR representations, early binder collection and error states, including multi-binder slots. Give every exhaustive match a deliberate arm; no new silent fresh-variable fallback.
4. Implement syntactic classification, lookup/pending scope, pattern validation, alias-cycle edges, forbidden contexts, summaries and the shared type visitor.
5. Add group ordering and diagnostics without altering SCC membership or runtime needs/init. Schedule dependency-ordered alias/module events.
6. Add trial state and provenance observation; implement closure/equality batches and shape checks on pre-batch snapshots. Test reverse links, pure-variable SCCs and preserved constraints before wiring alias reads.
7. Wire annotation instance/sharing, definition reads, kconv's inferred-shape conversion, whole-annotation places, recovery and cache completion. Eagerly check unused definitions.
8. Add deferred exports, Typed.alias_types, queries and canonical package comparison, including parameterized aliases containing a closed Of.
9. Run the T2 tests in §10. With T1 integrated, run spec tests and `scripts/dev-verify rust`; compare existing checker/interface goldens. Use the coordinator's heavy wrapper for **every** cargo invocation. No Rust implementation or compiler run is part of this design-note commit.
10. T3 adds record/Without closure and shape arms, label checks, shown-place checks and the §5.9 builder test. M1/M2 add or reconcile the Lisp/Definitions rows; M4 applies the expansion restrictions. Each track owns and runs its deferred tests.
11. Run std/spec with reversed independent-group tie breaking and with recovery enabled. Measure std time with the accepted T1 measurement procedure; investigate any regression. Audit WAT for existing programs and for type-only changes, allowing only differences justified by changed inferred runtime tests.

## 10. Tests and expected results

These are implementation requirements, not tests reported as passing. Each expectation names its governing rule. Run runtime/type-test cases at O0 and O2. Use complete project fixtures for module/export cases.

| Test | Program or change | Expected result and rule |
|---|---|---|
| Annotation instance | `id = (x) { x }`; `twice = (f: id, n: Int) { f(f(n)) }`; call with identity and 3. | Result 3; R1. Separate `id` occurrences admit separate instances; one annotated parameter remains monomorphic. |
| Open alias | `Same = id`; separately `g = id(id)` and `G = g`. | E0533 and E0534 respectively; R2/R4. |
| Local sharing | `f = (xs: List(Int)) { acc: xs = []  acc }`; call with `[1]`. | Result []; R1. A local alias of a weak captured value still shares its variables, rather than freshening them. |
| Inferred, not narrowed | K1's `copy` with `"a"` under Assert; tuple parameter with individually annotated parts. | Result `"b"`, no E0520; projected `q` has known Int; K1. Also test an unannotated outer parameter narrowed by a type test: its shape stays unknown, not the tested type. |
| Shadowed own name | An outer f contains an inner function with parameter f: Int and body `y: f = f`. | Allowed; the inner parameter identity covers the outer own-binding marker. In the inner header itself `(f, y: f)` remains E0536; V3. |
| Matched-value known type | `Box(A) { Box(A) }`; `f = (p: Box(Int)) { Box(a) = id(p)  y: a = "s"  y == "s" }`. | Result True; the non-generalized a holder is shared, while Of(a) has unknown shape. Do not copy p's known Int field into the inferred-shape conversion; R1/K1. |
| Expected known type | `list.map(["a"], (x) { y: x = "b"  y == "b" })`. | Result [True], no false E0520 from a source shape of "a"; K1. The inferred holder stays shared, and its unknown shape is erased. |
| Literal exactness | `port = 8080`; `Port = port`; `bad: Port = 8081`. | E0501; with source annotated Int, both values accepted; R6/T1. |
| Nominal extraction | §7.4's size fixture. | Size = Int; encoding an Int remains legal. Reading the untouched encode part is E0534; P4/M1. |
| Skip sharing | `s: PairType(Int, Int)` with two distinct fixed parts; separately a fixture with the same weak variable in both arguments. | Skip never allocates/copies a variable; a distinct skipped part remains live, a shared demanded variable is fixed in both places; P4. |
| Lower-bound graph | Hand-built weak v with a concrete predecessor only in `from`; a pure-variable cycle with one concrete Int bound; same cycle with none. | Int, Int, E0534; R4. No bound disappears. |
| Old constraints | In solver tests retain `v <: w`, fix v to 1, then constrain 2 <: w (allowed), and separately 2 <: v (fails). Include structural/invariant/env links and extrusion. | The first does not force w equal to v; the second retains the existing error. Fixing cannot detach the graph; R5. |
| Failed batch | Lower-bound substitutions that conflict with an upper bound; unsupported form; structural recursion. Compare solver state before/after failure. | E0534 with unchanged main solver, no assertion/panic or leaked failures; R4. |
| Shape traversal | Source root is a weak variable with lower bound Codec(D,E), not a direct Con. | Decode projection succeeds after the skeleton check; a later incompatible root member fails; skipped E stays shared; P4. |
| Fixation restriction | Invariant Cell fixture below, `C = r` then `later = () { r.set(2) }`. | Later mismatch with note at C; source annotated Cell(Int) permits it; R5/R6. |
| Earlier weak value | `c = cell(1)`; `get = () { c }`; `G = get`; unrelated `later = () { c.set("s") }`. | E0534 at G in either independent-group order; no fixation error in later; R3. |
| Consumer isolation | Module lib exports r/reset with `r = cell(1)`, reset sets `"s"`, and no lib type read. App or a test module adds `C = lib.r`. | E0534 only at the consumer; library check/interface unchanged; R3/X3. With a source annotation Cell(Int), a consumer read is allowed. |
| Scope | Own pattern/header/value examples V3; local m covers imported m in `m.x`; module-only `m`; private member. | E0536, E0207, E0205, E0204 respectively; V2/V3. Pending qualifier stays E0207 after adding an import. |
| Import-stable item | Add/remove an import of User around `Schema(User, _) = user`; add it around `Pair(A) = (A,A)`. | First remains a pattern and gains E0537 on collision; second remains generic; P1/P3. |
| Pattern shapes | Tuple through Pair alias; wrong head; union with Nil; exact head-alias part; duplicate binder; invalid macro part. | Correct projection; E0535; E0535; equality or mismatch without filling an unknown; E0208; E0532; P2–P4. |
| Eager order | Unused mismatching pattern; `A = List(B)` before a pattern defining B; module containing only an imported-source pattern. | Unused error is reported; A completes after B; no-group module still completes; P5. |
| Cycles | O3 example; self Of through alias; two type edges between groups; pure alias circle; import circle. | E0530 with chain for the first three, E0223 and E0214 for the last two; O2/O3. SCCs/needs/init unaffected on valid fixtures. |
| Recovery | Query all preceding bad-cycle fixtures and an unrelated valid binding. | No panic/fresh inferred alias; Failed propagates, unrelated hover/navigation survives; O4/X4. |
| Early places | `T = x` under constructor field, service/env parameter/statement, host and external declaration, also through multiple aliases. | One E0531 at the use; K3. A completed cached body does not evade the ban. |
| Whole places | `(f: pick)`; outer function with annotation `List(a \| b)` using distinct local variable holders a,b in an input place. | E0518; K2. Standalone Of checking must not miss surrounding unions. Retain T3's shown-type tests. |
| Hidden exports | Export a binder/alias/top annotation whose result contains private Secret; repeat through private aliases. | E0227 after completion; exporting only the source value is insufficient; X2. Private source with public resulting types is allowed. |
| Alias exports | Export User and User(..) in separate fixtures; attempt to use underlying private constructors. | Same exported alias, no extra constructor access; X1. |
| Package comparison | Change private source Int → String; rename it only; reorder union/record fields; rename alias formal parameters. | Exported alias changes only in first case; X3/T3 canonicalization. `T(A) = (A, port)` keeps A formal and port's literal exact. |
| Queries | Head/source/binder navigation, multi-binder outline, inferred/failed hover. | Existing head target, source binding target, individual alias targets; stable outline without inference; X4. |
| Macros (M2/M4) | Body macro yielding a value name in a generic alias; generated direct Of; generated alias from ancestor/non-ancestor/imported source. | No reclassification; E0531; accepted/E0531/accepted only after imported completion; P1/M3. Definitions returns the written item, not an inferred body. |
| Records (T3) | records.md §5.9 codec plus later encode calls; read encode side without lower bounds; read record key/label-parameter binding. | User = { age: Int, name: String }; encode calls work; E0534; E0543/E0542; M1/V4. |
| No feature | Existing std/spec without Of or patterns; reverse independent order. | Same accepted programs, canonical types, needs and init; no demand batches; O2. Measure rather than assume timing. |

A complete, ordinary i fixture for the invariant Cell tests (no host or mutable primitive is required to exercise invariant type constraints):

```i
Cell(A) { Cell(get: () -> A, set: (A) -> Nil) }
cell(A) = (x: A) {
  Cell(get: () { x }, set: (y: A) { Nil })
}
r = cell(1)
C = r
later = () { r.set(2) }
```

The setter may ignore its argument; its type still makes A invariant. This fixture tests typing restrictions, not mutable-cell runtime behavior.

## 11. Self-attack record

All programs here use the proposed syntax. “Checked by reading” means the indicated implementation path was inspected. No T2 compiler exists in this lane; acceptance/rejection below is a prediction of these rules, not a claimed executable result.

### Discovery pass: confirmed problems and corrections

1. **Soundness/constraints: replacing a pinned variable detaches existing relations.** With a hand-built constraint `v <: w`, a read fixes v while a later bound reaches it through w/structural relationships. The old proposal replaced variable operands and stopped walking their old lists. It supplied no proof that every propagated constraint survived. Reading `solve.rs:245–285,385–447,577–608` confirmed multiple propagation/direct-write paths. **Fix:** retain the original variables and add checked equalities; no solver replacement invariant (R4/R5). Do not assert that a candidate always checks.
2. **Cannot build: reverse bounds and pure cycles.** A variable can receive values through `from` without those being in its own lower list (`display.rs:531–538`). Literal-only collection can miss them; recursive DFS can reject a mere variable equality cycle. **Fix:** follow both edges and collapse pure-variable SCCs, failing only unfilled or structural cycles (R4).
3. **Cannot build: lazy completion misses empty modules and alias order.** A module containing only `Codec(Size, _) = imported.size` has no group; `A = List(B)` may be written before the pattern defining B from the same source. Source-order completion either never runs or reads Pending. `infer.rs:188–191` confirms the current group-only loop. **Fix:** module-start events and dependency-before-source ordering (P5/O4).
4. **Source conflict: alias exports.** The prior draft called `User(..)` an error. `resolve.rs:797–803` explicitly accepts it. **Fix:** preserve the alias export rule and add a regression (X1/§7.1).
5. **Soundness/known types: a fitted literal narrows a value fact without changing its type.** K1's copy program would reject its `"b"` annotation, or produce a false E0520, if kconv used narrowed kvars. T1 explicitly distinguishes these facts. **Fix:** pure inferred-shape conversion of stored monotypes/schemes; no fitted-literal or branch fact feeds value-type conversion (K1/§7.6).
6. **Cannot build: a pattern traverses a weak root while skipping an open argument.** Fixing the whole Codec closes encode unnecessarily; fixing only D leaves the observed root shape unchecked. **Fix:** retain observed-skeleton constraints with original argument identities, while closing only demanded leaves. A skip never creates an independent hole (P4).
7. **Known-type conflict: expected types can also be narrower than an inferred holder.** In `list.map(["a"], (x) { y: x = "b"  y == "b" })`, the callback's expected known parameter may be "a", while the shared inferred holder must also accept "b" from y's annotation. Using that expected literal for Of's known conversion would give a false E0520. **Fix:** no expected-known fallback for an unannotated source parameter. Only its stored structural monotype provides type shape; written source annotations are part of that monotype (K1). Add the corresponding test. This is distinct from ordinary callback expected-type checking.
8. **Scope conflict: a global pending-name set over-rejects shadowing.** `f = () { g = (f: Int) { y: f = f  y }  g(1) }` must read the inner parameter's type. Keeping the outer f text globally forbidden would give E0536. **Fix:** own-binding reserved frames participate in lexical type lookup and may be covered by inner binders; current-header pending names remain a separate check (V3).
9. **Source conflict: a tuple-pattern annotation in the inherited examples is not legal.** The old examples put a type after the entire tuple pattern. Spec §7.1 and `ilang-syntax/src/ast.rs:327–338` only allow a typed name/discard. **Fix:** use `((a: Int, b: String)) { q: a = a  q == 1 }`, annotating individual parts; no pattern-syntax extension is proposed.

### Attack pass 1 after those corrections

- **Soundness:** `Box(A) { Box(get: () -> A, put: (A) -> Nil) }`; extract from a weak Box then pass it to incompatible invariant contexts. R4 checks both equality directions with the original graph; later incompatible constraints still fail. A skipped occurrence of that same A cannot escape fixation. The expected-known callback counterexample is also rechecked with an unannotated holder: its shape stays unknown while its monotype is shared. The inner-f shadowing example follows lexical reserved frames. No further serious issue found by tracing `constrain_in`, `compare` and the proposed scope rules.
- **Buildability:** `A = List(B)`, then `Pair(B, _) = pair`, with pair's result behind a weak root and an unused encode-like part. Module/group events and dependency order complete B before A without forcing the skipped part. No Pending fallback is needed.
- **Sources:** `Schema(User, _) = m()` with a syntax macro returning `user`, and a later import of User. The stored generic classification gives a parameter-form error throughout; no import/expansion changes meaning. A newly generated whole pattern item is classified as such. This keeps the macro draft's syntactic definition service and accepted macro reservation. No further serious issue found.

### Attack pass 2

- **Soundness/order:** lib's `r = cell(1)` and `reset = () { r.set("s") }`; add `C = lib.r` only in app/test, and permute independent groups. The consumer cannot add equations; G from a captured earlier weak c fails even when it merely carries c without constraining it. Library typing remains unchanged. This assertion concerns completion/fixation, not the final display of other weak parts after ordinary consumer expression uses. No further serious issue found under R3.
- **Buildability/recovery:** `a = (n: b) { n }`, `b = a(1)`, plus a valid independent binding; also an alias-only module in an import cycle. Failed sites and prior-error state are set before query inference, so no new variable or missing-ready panic is required. Checked against `query.rs:26–48`. No further serious issue found.
- **Sources/interface:** export an extraction alias with `(..)`; rename its private source without changing the inferred type; then change the source's literal set. X1/X3 preserve constructor privacy, ignore the rename and detect the type change. T3 label binders/key parameters still fail before inference; record closure uses its exact rewrites. No further serious issue found.

These are two dry passes of design reasoning, not independent reviews or a solver proof. Implementation/RF must still execute §10, especially the rollback, alias-only module, weak-root projection and permutation tests.

## 12. Risks and coordinator questions

1. **Approve explicit fixation at completion? Recommended: yes, with the exact §7.5 wording.** Reading a weak type can reject later widening. This is the substantive semantic decision inherited from the draft, now made ordinary checked constraints. If G3 rejects that cost, use design C and require a closed source annotation; do not silently publish a mutable snapshot. C would need reconciliation with records.md §5.9.
2. **Confirm the early-context limit? Recommended: retain E0531 for T2.** A nominal wrapper around an extracted User is therefore unavailable; a structural alias works. Supporting such fields later requires ordering field-dependent inference and revisiting variance and macro readiness, not just an extra conv arm.
3. **Confirm T2's code block and file scope? Recommended: E0530–E0537 and the solver/checker/export/interface work in §8.** T2's plan file list alone understates the required changes. Do not change other lanes' code or notes in this note-only task.
4. **Accept the M4 boundary? Recommended: yes for the first version.** Direct value types in expansion are rejected; already completed dependency-safe aliases work. Coordinate the update on lane/note-meta before M4, including imported/module-start aliases and preserving the blackbox rule.
5. **Extend parameter or pattern forms now? Recommended: no.** Keep own-header reads E0536; defer left-to-right parameter types, function/record type patterns, union matching, macro-generated pattern parts and consumer fixation. Each needs its own inference/readiness rule. Tuple extraction and renamed heads already work.

The largest implementation risk is proving that trial batches preserve all prior relationships and roll back completely. Snapshot copies may be expensive for large sources; no timing or memory bound was measured here. Open-group rejection is deliberately coarse and can reject a disjoint read in a group that touches an earlier weak variable.

Verified here: source/branch/status reads, the listed code paths, accepted note/ledger contracts, section/fence/path checks and git diff --check. Not verified: new-syntax compilation, proposed diagnostics, solver transactions, the T3 builder, macros, query recovery or package comparison under T2. Those facilities are not implemented by this note. No cargo build, runtime test, performance result or formal principal-type proof is claimed.
