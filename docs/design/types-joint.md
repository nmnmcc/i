# Joint design: literal types, value types, and records

Status: reconciliation for G3. This note changes no compiler code and does not replace the three accepted designs. It supplies their shared contracts and corrections. Only this file belongs to this lane.

The source tree read here is `143c1c42fb0c2f8b22b5b190573f454db9dfd50b`. Code references without another revision mean that tree; paths beginning `types/` mean `crates/ilang-check/src/types/`. Bare solver filenames also refer to that directory; bare `hir.rs`, `resolve.rs`, `order.rs`, and `query.rs` refer to `crates/ilang-check/src/`; `package.rs` and `lower.rs` refer to the full paths in §2.3. Rebase by function name as well as line number. A1 and K1 are not in this tree. The macro draft was read with `git show lane/note-meta:docs/design/meta.md`, at `44021cc52e7fc0d3b3c6646067d29f81157c577d`. K1 was read, not executed, at `lane/k1` head `6b1c1ebebd07b95b7b8b200e4d391be7737eb2fc`, including its implementation commit `dc4ce6c`. A final ledger read reports K1 landed at `578a17d`, with coordinator-run Rust/i verification and equal WAT; those runs were not repeated here.

Sources, in precedence order: `docs/spec.md`; accepted `syntax.md`; accepted `literal-types.md` (T1), `value-types.md` (T2), `records.md` (T3), and `a1-front-end.md`; the plan `/home/agent/.claude/plans/twinkling-swinging-perlis.md`, “已定的设计”, D-types, and wave 2; the binding coordinator ledger `/home/agent/Programming/i/.rd/cache/ledger.md`. The ledger resolves choices left by those texts, including closed-only foreign reads and K1 before T1. The macro draft cannot override an accepted syntax rule.

## 1. Problem and choice

The individual designs leave incompatible replacements of rule 13, overlapping subtype rule numbers, a stale type grammar, and incomplete interfaces between known types, permanent aliases, record spreads, display, and code generation. Adding their match arms independently would compile while losing constraints or changing package identity.

Criteria are: preserve the accepted sets of values and principal inference rules; preserve distinctions between inferred types and known facts; use one representation at each boundary; require no schema, codec, or record-library recognition; allow the required merge schedule; leave macro syntax reserved without reinterpretation.

Three integration designs were considered before choosing:

| Design | Benefit | Reason to accept or reject |
|---|---|---|
| Separate track adapters: each track retains its own conversion, display, and testing helpers | Few edits during parallel work | Reject. A literal can disappear when T2 reads display text, and T3 can rebuild a whole String or reinstate the old known bound. Independent parsers can classify the same alias differently. |
| One expanded universal type node holding literals, value references, records, and unresolved macros through solving and lowering | One visitor and one broad representation | Reject. A value reference is an instruction to obtain a type, not a runtime head; an unexpanded macro has no type. This mixes readiness with subtyping and invites new solver rules. |
| Staged resolution into one solver algebra, with exact completed results and separate known facts | Keeps each accepted feature and exposes every shared boundary | Choose. HIR retains written syntax/provenance; T2 resolves reads before conversion needs them; solving receives only types; runtime tables contain only validated decisions. |

Keep the first design’s small independently testable stages and the second’s exhaustive visitor audit. Do not keep independent semantics or unresolved solver forms. This is reconciliation, not higher-rank types, type functions, lacks constraints, or a new inference algorithm. Literal sets serve protocols, finite state matches, and keys; value reads serve schemas, parsers, and callbacks; records serve data, configuration, and builder results. Each remains a general feature.

## 2. Agreed representation and ownership

### 2.1 HIR and resolution

The existing HIR has `Named`, `Var`, `Hole`, `Tuple`, `Fn`, `Union`, `Inter`, `Diff`, `Error`, and aliases have ordinary TypeIds (`crates/ilang-check/src/hir.rs:167–192,273–296`). Keep them and add, in order:

```rust
// Contract sketch; auxiliary identifiers live in the owning track.
Lit(LitValue),                                             // T1: Int or decoded String
Of { value: ValueRef, site: OfId, projection: Option<ProjectionId> }, // T2
Record { base: Option<Box<Type>>, parts: Vec<RecordTypePart> },     // T3
```

`LitValue` has Int and String cases only. T1 shares canonical numeric/string conversion with its set and coverage code. HIR may hold BigInt; the solver’s Int set keys remain canonical decimal strings. `RecordTypePart` is a field with a resolved label and Type, or a forgotten label. Keep each label span. A HIR label is a decoded name/string or a reference to a declared label parameter, never an inferred string expression.

`Of` is not a `Ty` variant or a `Head`. It resolves to a scheme instance, a shared monotype, or a completed closed definition result under T2 R1–R7. `ValueRef::Local` and `Def` already distinguish its two sources (`hir.rs:349–357`). A constructor is not an Of source. Each extraction binder stays `TypeBody::Alias(Of(...))`; a separate written-item/pattern record holds the common source, head, skips, binder spans, and dependencies. Projection ids identify one completion operation, not a reusable partly open source cache.

`TypeCall` remains an AST/core form. Before M2, resolution reports E0234 and uses HIR Error for recovery; it must not resolve the head as an Of, instantiate a function, or create an alias. After M2, expansion removes it before ordinary type resolution. T1 changes A1’s type-call arguments to types, since literals are now types. It does not implement macros.

The one `resolve::ty` dispatch is:

| Syntax | Dispatch and owner |
|---|---|
| Int/String literal, negative Int through groups | T1: `Lit`; Float is a parse error |
| Unqualified UpName | A1/A3: declared type variable first under its collision rules, otherwise named type; arguments on a variable remain E0230 |
| `m.T(...)` | Named type, normal module/export/arity lookup; T2 does not capture it |
| Lowercase `x` / `m.x`, without joined `(` | T2: pending/own-binding checks, ordinary value lookup, `Of`; `m` must be a module in the qualified form |
| Lowercase `m(...)` / `a.m(...)` | Reserved macro form, regardless of imports |
| `{ ..T, ... }` / `{ label: T, ... }` | T3: record type, decoded duplicate check, label binding checks; recurse into base and fields |
| `{ T }` | Group; no distinct inferred type |
| Remaining structural forms | Recurse through the same visitor; propagate context and Of provenance |

The old dispatch and lookup are at `resolve.rs:1329–1407`; service and host checks at `:1445–1467,1568–1660`; alias expansion/substitution and visitors at `:1409–1438,2630–2697`. T1 adds literal leaves there, T2 adds reference/context/dependency handling, T3 adds base/field recursion and label uses. The shared visitor must distinguish type children, value references, label references, and pattern binders. A literal or label text has no dependency. A record base or field can contain an Of dependency; a forgotten label cannot.

Keep A1’s UpName frames and T2’s identity-based pending-name and own-RHS frames. Run T3’s reusable L1/L2/L2a/L5/L6 checks over the resolved references, including Of and alias-pattern sources. A same-header read first reports T2 E0536; an otherwise legal read of an outer key is E0543; a read of a label-generic callee is E0542. Errors do not authorize inference through a placeholder alias.

### 2.2 Solver types and public tables

The present solver separates `Head` from `Ty`, stores `Con(Head, Vec<Ty>)`, and identifies heads through `head()` (`types/mod.rs:40–102`). The joint form is:

```rust
Head = Named(TypeRef) | Tuple(u32) | Fn | Record;
Ty = Var(V) | Con(Head, ValueSet, Vec<Ty>) | Fn(Box<FnTy>)
   | Rec(Vec<(Label, Ty)>) | Without(Box<Ty>, Vec<Label>)
   | Union(Vec<Ty>) | Inter(Vec<Ty>) | Diff(Box<Ty>, Vec<Ty>)
   | Never | Top | Both(Box<Ty>, Box<Ty>);
Label = Name(Arc<str>) | Param(LabelParamId);
```

These are notation, not Rust declarations to paste. T1 adds ValueSet; T2 adds no Ty/Head; T3 adds Record/Rec/Without. Non-All sets occur only on argumentless Int/String Con nodes. `Con(Record, ...)` is invalid. Rec fields and Without bases are covariant; a nested invariant constructor is still invariant. Only smart constructors normalize Rec/Without. No reconstruction defaults an existing Con’s set to All.

Three head questions must remain distinct:

1. `head()` identifies a mergeable concrete member: Con, Fn, Rec. Without has none.
2. `value_head()` additionally identifies Without and a spread intersection as records when testing possible runtime overlap. It is not permission to merge them.
3. `is_spread()` recognizes Without and intersections containing it. Routing uses T3’s priority rules, not “first member with this value_head”.

A union has at most one ordinary member per head, including one Rec, but may also retain spread members. Int/String splitting is T1’s finite/cofinite algebra, never a rule for records. Records carry implicit All, not a set of fields in ValueSet. Source: T1 decisions 2–3; T3 D1–D3 and §§4.2–4.5.

The public result tables currently keep span-keyed access, argument order, heads for type tests, and may-fail decisions (`types/mod.rs:148–199`). Their final contracts are:

| Table/form | T1 | T2 | T3 |
|---|---|---|---|
| `type_tests[(ModuleId, Span)]` | `Vec<TestMember { head, values: TestValues }>`; All/Only/Except | Fully expand completed aliases before producing members | Record is `TestMember { Record, All }`; no fields in the runtime test |
| `scheme_types`, `schemes` | Full literal sets, never shortened | Ordinary scheme instances remain separate from fixed definitions | Full record/spread types and label-parameter metadata |
| `alias_types[TypeId]` | No new table | Full ShownType of each completed value-dependent alias/extraction binder, with formal identities | Record/Without output through the same exact writer |
| Internal completion state | Preserve sets | Pending/Ready/Failed sites, closed fixed forms and provenance; trial state is private | Closure over fields/base; no labels escape |
| `access` | Preserve existing access kinds | Of has no runtime access | Add `Access::Label`; retain Field/Part |
| `args` | Preserve permutation | Type dependencies add no runtime arguments | Preserve permutation and written evaluation order |
| `key_labels` | — | — | Call span to parameter indices and `Name(string)` or `Param`; populated only after successful L4 validation |
| `may_fail` | Deferred per-binding Assert decision | Completion must not precede pending group checks | Records do not introduce record patterns |

M2 later changes all span-keyed tables together to expansion-safe node identities, including T3’s key table. T1–T3 neither invent an independent macro key nor treat an alias TypeId as a runtime constructor.

### 2.3 Shared-file implementation map

Every row below is a shared region. “No addition” means preserve the earlier track’s change. Line ranges describe code read in the baseline, not a claim about a future implementation.

| File and read region | One final responsibility; contributions in T1 → T2 → T3 order |
|---|---|
| `crates/ilang-syntax/src/ast.rs:371–406` | On A1's nodes, T1 adds literal leaves and simplifies TypeArg → T2 preserves ValueOf/pattern classification → T3 adds Record/base/parts. Keep attribute/item macro nodes from A1. |
| `crates/ilang-syntax/src/parser.rs:452–531,1150–1291` | One type-primary dispatch: literals/negative groups → lowercase value reads and syntactic alias-pattern classification → records versus groups. Preserve TypeCall and the complete §5 lookahead contract. |
| `crates/ilang-syntax/src/print.rs:528–629`, `format.rs:1472–1552` | Literal/negative type text → Of and alias-pattern text → record parts and required first-field comma. All use the same precedence and exact-tree rules. Here `format.rs` is in the same syntax directory. |
| `hir.rs:167–192,273–296,349–365` | Lit → Of/pattern metadata → Record/labels. Keep aliases as aliases. |
| `resolve.rs:1329–1467,1568–1660,2630–2697` | Dispatch/visitors above; directional literal ABI checks → Of readiness/context and deferred exports → labels/record rejection at service and host boundaries. No catch-all that hides a new type. |
| `types/mod.rs:40–102,133–210` | One algebra and the additive tables above. T2’s solver snapshot remains private. |
| `types/solve.rs:245–345,385–633,698–908` | Literal algebra and bound fusion → transactional checked completion/open-group observation → record arms, width, spreads, escape checks. Preserve T1 sets and T2 rollback on every T3 mutation. |
| `types/display.rs:215–244,515–615,698–755,860–910,998–1053` | Exact literal normalization → fixed-form reads without live display fixation → Rec/Without normalization and label names. Keep K1 in every occurrence walk. |
| `types/shown.rs:10–31,43–76` | Lit and full/limited writer → alias bodies with formal identities → structured labels and spreads; shared canonical writer contract in §4. |
| `types/known.rs:22–69,102–162,239–270,322–418,434–510` | Set-sensitive members, hidden inputs, whole_lits, known_bound → pure inferred-shape Of conversion → record-aware safe known summaries and label substitution before matching. |
| `types/pattern.rs:157–230,460–482,563–617,723–757` | Literal/fitted distinction, set narrowing/tests and deferred coverage failures → convert Of using its site policy → Record/All tests and `{..}` taken head, not Con(Record). |
| `types/coverage.rs:113–236,615–630` | `(Head, ValueSet, Vec<Ty>, Mode)`, finite/cofinite usefulness → only completed inferred source types, no Of case → `(Record, All, [], mode)`, record witness text. |
| `types/infer.rs:233–355,603–687,852–1005,1260–1347,1538–1678,2108–2120` | Literal inference, call known bounds, deferred Assert → ordered completion and Of instantiation → records, L4 call preparation, shown-place checks. See §3 for exact group/call order. |
| `order.rs:17–131,188–251` | Preserve expression graph → second graph over existing SCCs and completion events → walk record expressions for runtime uses and record types for type uses; reject multi-binding label recursion on the expression SCC. |
| `query.rs:342–346,397–461,1218–1241` | Literal text/leaf navigation → source references, binder targets, completed alias hover → record fields/label references and Access::Label completion. Source spelling and inferred hover are separate views. |
| `crates/ilang-wasm/src/lower.rs:1500–1513,1675–1685,2123–2135,2209–2237` | TestValues equality/tag conjunction → assert no unresolved alias test head → Record tag test, access/literals/spreads/key labels, empty Field list trap. |
| `crates/ilang-cli/src/commands/package.rs:1063–1104,1159–1182,1235–1385` | Full literal identity → completed alias bodies instead of source value names → record/label canonicalization. One interface path, detailed below. |
| `crates/ilang-check/src/lib.rs:18–98`, `crates/ilang-syntax/src/lib.rs:18–71` | Keep existing codes; apply the disjoint allocation in §7. Retired reservations are not recycled. |

The solver currently propagates bounds in both directions, stores reverse links, and mutates bounds during extrusion/freshening (`solve.rs:265–285,385–447,577–633`). Therefore T2’s trial must include every field of solver state, not just failures or the initial variable. T3’s record comparisons and label-level checks must use that same observed state. T1’s literal bound fusion is limited to argumentless Con; it must never deduplicate Rec upper bounds by their head.

The package writer currently prints inferred value trees but written alias bodies (`package.rs:1100–1104,1321–1323`). That is the concrete reason T2 needs a completed alias table; adding a pretty-printer Of arm that prints its source is insufficient.

## 3. Rules at the boundaries

### J1. Inferred types, known types, and keys are three separate uses

T2 K1 says: “Of reads inferred schemes/monotypes, never a narrower flow fact.” T1 decision 7 says: “Known types must not have fewer values than may come.” T3 L4 says that a known singleton chooses a label and the solver then checks the argument against that singleton. Keep all three sentences.

```i
copy = (s: String) {
  x = "a" = s
  r = ( value: x )
  y: x = "b"
  (r.value, y)
}
```

`copy("a")` gives `("a", "b")` and needs Assert for its fitted pattern. `x` and the inferred field type of `r` are String; their known facts can contain `"a"`. `y` uses String. `record.get({ a: 1 }, x)` chooses label `a` from the known fact but fails E0501 because the inferred String is not a subtype of `"a"`. This is an intentional restriction of L4, not a reason to read known types in Of or strengthen rule 13. Use a literal argument or a genuinely singleton-typed binder.

Rule 12 narrows the *new catch-all’s* inferred type. Thus a catch-all after `x: String - "a"` can have inferred and known type `"a"`, be read by Of inside the clause, and be a valid record key. The original subject name is not narrowed to a literal by that test. T3’s statement that matching never helps keys must be limited to the original subject name, not new binders. No dependent relationship is introduced between an arbitrary String value and a label.

T3’s L2a/L5 bans remain even though T2 reads inferred types: a direct read of a key parameter is E0543 and of a label-generic function is E0542. A local ordinary copy of the key has String, cannot carry a rigid label, and cannot be forwarded under L4’s own-name rule. Type aliases never bind a new label parameter.

### J2. Rule 13 keeps T1’s bound in nominal access and tuple access

T3 replaces rule 13’s dispatch, not T1’s bound semantics. In rules 13a and 13c, and in calls constrained from a known function shape, constrain against `known_bound(K)`, not raw K. `known_bound` has T1’s polarity rule: widen Int/String at covariant places, drop partial-literal exclusions there, and use fresh variables at noncovariant places. T3 adds covariant Rec fields and Without bases. Rule 13b has only its principal constraint `t <: { f: A }`; it must not additionally constrain the target or field against a narrowed known fact.

The inferred field/tuple result comes from those same bound forms (or the existing §7.5 constructor-pattern arguments), while its known result comes separately from the original known facts. This follows the existing split at `infer.rs:1636–1678`; widening only the target constraint and still returning a raw known singleton would let Of accidentally read a flow fact. This prevents the false String-to-singleton failure in J1, also for `Box(x).value` and a tuple holding a record. It preserves the rejection of an actual call `sink.put("x")` when sink’s inferred parameter accepts only `"GET" | "POST"`. Source: T1 decision 7 and §5.4 override the stale “subtype of its known type” sentences in T3 §§6/12.

Known record summaries are conservative. They are not the solver’s exact types. For Without, discard non-record alternatives, forget labels that are equal or may be equal, and use an all-record summary for an unknown base. Do not return Int from `members(Without(Int, L))`, or an unknown non-record head from an unknown base. Normalize first where possible. If several record alternatives survive (Rec alongside spreads), combine their guarantees by the record join, keeping only common guaranteed fields. `member`/`one_member` must never select an arbitrary alternative with extra fields. Unknown alternatives erase unsupported field facts. Summarize only the record alternatives: preserve non-record and unknown alternatives in the enclosing known union, so `Top | Rec` never becomes records only. The same summary is used for field known types, expected fields, and disjointness. It preserves T1 sets in fields that remain known.

For `match_k(Rec, K)`, a missing known field supplies Top to variables in the expected field rather than proving that no values arrive. This applies T1’s unknown-part rule to T3’s new structure. Field constraints are still enforced by ordinary inference. `whole_lits`, `hidden_inputs`, `known_bound`, `note_general`, `ksubst`, and Of inferred-shape conversion must all visit Rec and Without. None may fall back to an old head-only filter. T2’s local inferred-shape conversion never follows temporary bounds or `kvars`.

### J3. Calls have one inference order and the existing runtime order

The current call code builds a known callee, places arguments, infers two passes, then records parameter-to-source indices (`infer.rs:1260–1342`). After T3:

1. Resolve the callee and use its parameter labels to place arguments, including the piped first argument. This needs shape/labels, not an instantiated rigid record label.
2. For a label-generic callee, infer each key argument once, with expected String, before other arguments. Pick labels by L4 and check repeated keys agree. Keep its `(type, known type)` result; do not infer it again in the later passes.
3. Instantiate the scheme and rebuild its known function type with the *same* label map. A concrete key slot becomes its singleton String type; a forwarded outer key slot becomes String. Check the concrete key argument against its singleton. Use T2 ordinary instance/sharing rules for any Of inside allowed annotations.
4. Compute T1 hidden inputs on that substituted known function. Infer the remaining arguments in the ordinary two passes. Apply whole_lits to the hidden substitutions for every expected argument and the final known result.
5. Record type tests/accesses and, on successful key checks, `key_labels`. Lower arguments in written evaluation order, using `args` only to place their results. Inference order is not runtime evaluation order.

A nonliteral expression with singleton type may have effects. Lowering must evaluate it once before replacing its resulting key value with the shared label constant. Label parameters do not create a special call path for `record.get`; the same process applies to every ordinary binding that declares one. Source: spec §6.2; T1 decision 7; T3 L4 and §9.3.

### J4. One group lifecycle; no fixation through display

The existing group loop allocates holders, infers values, links holders, generalizes, fills known types, and checks coverage (`infer.rs:233–355`). The joint order is:

1. Record the solver allocation boundary before holders. Infer and link the ordinary expression SCC; no type dependency enlarges it.
2. Resolve T1’s deferred binding Assert variables in their saved effect frames after holder linking and before generalization. Flush ordinary failures and occurs checks. A binding’s Assert failure is E0501 at that binding, as the ledger requires.
3. Decide generalization with K1 preserved; finish ordinary known/coverage checks. T3 checks the shown type of each binding, also a non-generalized binding, for illegal spread input unions. This check does not fix variables.
4. A failed source group publishes Failed. Otherwise take one immutable snapshot of its schemes and bounds for all own-module demands. T2 computes closed candidates without adding bounds; T3 normalizes closed Rec/Without candidates with exact rules. Skips demand no contents, but retain source variable sharing.
5. Trial all fixation and demanded-shape constraints together; enforce occurs, places, open-group ownership and T3 label escape checks. Commit only a successful batch. Recheck resulting forms/places before publishing closed aliases and any wholly closed source form. Recompute affected derived known/coverage results if needed; never keep a decision made from fewer possibilities or publish a result while a deferred failure remains.
6. Assemble dependent aliases in dependency order, including module-start aliases when the module has no groups. Discard partial snapshots/skeletons. Later/foreign extraction uses T2 R7’s closed source gate before projection; it cannot restart the trial.

This retains T2’s explicit language commitment: a demanded weak type is fixed by checked equalities, not by calling display. A declared ordinary type variable stays flexible (spec §4.7: “A type variable in an annotation may become a more exact type.”). Label parameters alone are rigid labels. T3’s schema example may infer `R <: {..}`. No new rigid ordinary type parameters are part of this task.

`Codec(User, _) = user` in the builder’s own module fixes the decode part and demanded Codec shape, leaves a distinct open encode variable live, and exports User. A consumer imports User. It cannot perform a new partial projection through open `codecs.user`, even if decode is already fixed. A second demand that reads the unfilled encode part fails E0534; it is not silently Never. A decode alias is stable because fixation preserves checked equations, not because every future expression use is incapable of constraining it. Source: T2 R3–R7/M1; ledger G2(S6). T3 §5.9’s “whatever the rest of the module does” is too broad.

### J5. Solver, tests, and coverage share membership rules

Keep T1’s exact split for Int/String: send the portion not covered by the same-head finite/cofinite member to the one eligible variable-like alternative. Keep T3’s record routing: the exact Member shortcut; unions/right intersections; variable arms; R-split; R-without-right; R-spread-union by label inclusion; R-without-left; left intersections; difference/routing; R-width; remaining nominal/function arms. These are T3 §4.4’s order, with T1’s set rules inside the nominal/difference arms and T2’s mutation observation around all of them.

Runtime overlap uses the actual domain: Int/String need overlapping sets; Rec/Without/spread values all have Record; nominal values and records are disjoint. `head()` alone is insufficient for this question, while `value_head()` alone is insufficient for merging or routing. In particular T1’s fallback must not mistake a record-only spread for a universal variable alternative for the rest of an Int set.

A record type test takes all records. Its fields are an inference constraint, not field tests. For a subject `{ a: Int | String }`, a test `x: { a: Int }` with a catch-all must fail; it cannot select the Int-valued records. A type test of a closed T2 alias expands first; of a record alias it makes Record/All, of `"a" | "b"` it makes String/Only. A spread test is Record/All even though its solver head is absent. Ordinary unresolved variable members still give E0511.

Coverage records `(Record, All, [], mode)` and treats its runtime test as `Pat::Head(Record, 0)`. Never use an empty finite set for records: that would erase a possible runtime value. Rule 12 takes `{..}` after a covering record test. It subtracts literal sets for Int/String. A record containing an empty field can conservatively remain a record in coverage; that may reject more, never claim coverage of a possible value. No record patterns or record-shaped alias patterns are added.

T1’s deferred `FailKind::Values` applies only to value-set failures inside rule 11. A record field mismatch is an ordinary failure; hiding it merely because coverage sees Record would be unsound. Nested literal failures may still be held, but are dropped only if the same match actually reports E0516; otherwise flush E0501. A one-clause block with choices is fitted and can need Assert; it does not become an exhaustive match. T1 owns removal of A1’s E0235 and the `tests/spec/6/one_clause_choices.i` gate.

### J6. Early contexts and the host boundary compose

T2’s context check precedes host/service shape checks: a value-dependent alias in a constructor field, external/host declaration, or environment type gives E0531, even when its completed body would be an allowed Int. Retain dependency provenance after completion, so expanding an alias cannot hide this restriction. Reading an ordinary function scheme with an existing environment is allowed; writing a new Of in that environment is not (T2 K3).

T1 then applies direction to allowed Int/String types: host-provided values must be whole; i-provided values may have any set and are represented by their ordinary ABI type. T3 rejects records in either direction and rejects label-generic entry points even when the ABI would otherwise contain only Strings. No new ABI record conversion is implied by T2’s alias table. These are independent restrictions, not an override of T1 by T3’s new §10.3 paragraph.

### J7. K1 is a prerequisite, not a replacement typing rule

At this baseline `generalize` calls `display::simplify` and may rebuild its result (`infer.rs:2108–2120`). Coalescing creates Both for invariant arguments (`display.rs:558–573`), but occurrence analysis walks both directions only for environments (`:1028–1038`). K1 changes that guard to `env || v == Variance::Inv` (`lane/k1`, `display.rs:1030–1038`). This is why a seemingly cosmetic display rebase can lose a principal type variable.

T1 must preserve that condition, variable identities, polarity, and weak levels while adding the set field; a set has no variables. T2 must instantiate the preserved scheme, never infer that an erased/shown Never is a closed variable, never generalize a weak variable by reading it, and never use known-type widening as permanent closure. T3 must traverse fields/base with surrounding polarity, still count invariant children in both directions, preserve labels separately from solver variables, and retain the constructor-depth occurs check for both new forms. All three must run K1’s positive and rejection regressions after their shared-file rebase. Required order is K1 → A2 Namer → T1 → T2 → T3 for shared display work.

### J8. Exact forgetting must survive T2 closure and substitution

There is one remaining mathematical conflict inside T3 that becomes observable at T2’s permanent-read boundary. T3 §4.3 calls E4 an equality even when a removed field contains an unresolved variable, and says “E4 applies to it”. But with `R = { a: A, b: Int }`, erasing a gives `{ b: Int }`; substituting `A = Never` first makes R empty and forgetting must give Never. The two orders disagree. The “no record is ever made” justification is insufficient for a type alias or a type in an input place: types describe sets independently of whether a current expression constructs them. Spec §4.1 and T3’s own set definition decide this.

The conservative joint correction is: apply E1 before E4; E4 may discard a field only when its type is known to have at least one value under every remaining substitution. Otherwise retain Without. No new nonempty constraint or inference choice is introduced. For the first implementation use one fixed conservative predicate, after ordinary exact normalization: true for Top, a nonempty Int/String value set (including All/Except), a tuple whose parts all pass, and a Rec whose labels are distinct fixed Names and whose fields all pass (including `{..}`). A union passes if a member passes. All other forms return false. Do not infer inhabitation from a variable’s current bounds, from each side of an intersection separately, or from individually inhabited fields whose parameter labels may later coincide. Retained fields need no such proof because they remain in the result. The existing label-equality conditions still apply. T2 closes dependencies before attempting this rewrite, and never freezes the result of an earlier open-field erasure.

This loses some optional simplification, not the required builder: in the accepted Schema/Codec decode example each overwritten known field is String or Int, and its closed result can still normalize. Generic spreads can remain Without and use R-without-left/right. The exact impact on inferred schemes must be tested before T3 lands. Coordinator question Q1 below asks approval for this correction to the accepted T3 algorithm; the open-variable E4 rewrite must not be implemented while that decision is pending.

## 4. Full text, canonical identity, and queries

One value can have three presentations, with explicit consumers:

- Written HIR text: queries showing an annotation retain `x` or `m.x`; navigation targets that value. Alias-pattern navigation targets the existing head and each new binder separately.
- Full type tree/text: simplified schemes, completed aliases, `i check --types`, package comparison, and future Types syntax. No truncation, private value spelling, solver id, provenance, or fixation site in type identity.
- Limited human text: diagnostics and ordinary hover may shorten a literal set over 12 items to its first 10 plus the count. It must not be read back or used by a type operation.

T1 adds Lit; T2 adds completed alias bodies; T3 adds Record/spread and label identity. A `ShownType::Record` cannot use undifferentiated strings for labels: `{ "K": Int }` is a fixed string label, while `{ K: Int }` refers to a parameter. Use a structured shown label (`Name(decoded_string)` / `Param(identity, shown_name)`), or an equivalent lossless representation. Carry an ordered list of declared label-parameter identities on the scheme. A key slot also needs its identity: body inference treats it as String but a public scheme must show `key: K`, and instantiation must still find that slot. A separate key-slot map is sufficient; no Ty::Key is needed.

The Namer reserves all label-parameter spellings before naming ordinary variables. A canonical interface maps ordinary variables to internal `'0`, `'1`, … and label parameters, in their scheme declaration order, to `'k0`, `'k1`, …, as accepted in the ledger. These internal names are not source syntax. First substitute canonical label identities, then sort record fields by those identities, then name ordinary variables on that canonical traversal (retaining alias formal parameter positions). Sorting by source spelling and renaming afterwards would make a rename from K/J to Z/A change the interface. Preserve the same identity in key slots, fields and forgotten-label lists. Fixed string labels are never alpha-renamed.

Package `TypeShow` uses `Typed.alias_types` for value-dependent aliases, with their own formal parameter positions and canonical nominal TypeRefs. Ordinary aliases and constructor fields retain the written-type path, extended for literals and records. They need no Of runtime fallback: an unexpected unresolved Of here is failed readiness, not `_`. Normalize literal spelling/sets, decode and sort labels, and use full text on both paths. The package layer may report a type change under its existing policy; this task does not invent semantic-version subtyping compatibility.

The following are equal interfaces: `0x10` and `16`; `"\u{61}"` and `"a"`; a record’s field order changed; private source renamed without changing its closed read type; ordinary/label parameter alpha-renaming. The following differ: a changed literal hidden beyond the tenth displayed item; added/removed record requirement; fixed label `"K"` changed to `"J"`; a private source’s closed exported alias changing from `1` to `2`. A wholly closed record read uses the same literal sets as a written record type, not String/Int widening.

## 5. One type grammar, including macro reservations

This is the replacement grammar for spec §3 after all three tracks. The existing precedence and arrow rules remain. TypeArg is deliberately just Type; the old `Type | int_lit | string_lit` alternatives overlap after T1.

```ebnf
Type       = Union .
Union      = Inter { "|" Inter } .
Inter      = Diff { "&" Diff } .
Diff       = Prim { "-" Prim } .
Prim       = FnType | TupleType | NamedType | TypeCall | ValueType
           | Literal | "_" | "{" Type "}" | RecordType .
FnType     = "(" [ ParamType { "," ParamType } [ "," ] ] ")"
             "->" Union [ "?" Union ] [ "!" Union ] .
ParamType  = [ name ":" ] Type .
TupleType  = "(" [ Type { "," Type } [ "," ] ] ")" .
NamedType  = [ name "." ] UpName [ "(" Type { "," Type } [ "," ] ")" ] .
ValueType  = [ name "." ] name .
TypeCall   = [ name "." ] name "(" [ TypeArg { "," TypeArg } [ "," ] ] ")" .
TypeArg    = Type .
Literal    = int_lit | "-" IntPart | string_lit .
IntPart    = int_lit | "{" IntPart "}" .
RecordType = "{" ".." "}"
           | "{" ".." Type { "," RPart } [ "," ] "}"
           | "{" FPart { "," FPart } [ "," ] "}" .
RPart      = Label ":" Type | "-" Label .
FPart      = Label ":" Type .
Label      = name | string_lit | UpName .
```

The grammar is deterministic with these existing or accepted lookaheads; it is not claimed to be LL(1):

1. At a lowercase head, consume the optional one-part qualifier and final name. A joined `(` chooses TypeCall; otherwise ValueType. No import lookup changes this decision. Only a final UpName chooses NamedType. There is no postfix type application of `{ x }`.
2. After `{`, `..` or `Label :` chooses RecordType. Otherwise read exactly one Type and `}` as a group. An empty `{}` is invalid in a type; it is an empty clause block only in an expression. A record with one field is not a group.
3. After `(`, the matching `)` followed by `->` chooses FnType; otherwise TupleType. A function type consumes its result union greedily; `?` and `!` attach to the nearest arrow, in that order. Labels are allowed only in the parameter reading.
4. A `-` where a Prim starts requires IntPart. After a complete Prim it is difference. Thus negative literals are not lexer tokens. The braces in IntPart contain only an unsigned integer, recursively grouped.
5. Breaks inside a record type separate parts as commas; a break directly inside a type group remains an error. Joined calls use §1.7’s line rule. Delimiter lookahead respects nested braces/parentheses, not textual searches for a colon or arrow.

| Hard sequence | Unique reading / verdict |
|---|---|
| `x`, `{ x }`, `m.x` | Value type, grouped value type, module value type; field access on a value qualifier is E0207 |
| `x()`, `a.m(Int, -{1}, { x: "a" })` | Macro reservation E0234 until M2; arguments are types |
| `m.T("a")`, `A(Int)` | Named application; second is E0230 if A is a declared variable |
| `{ "a" }`, `{ "a": x }`, `{ K: "a" }` | Grouped literal; fixed-label record with an Of field; parameter-label record subject to L1/L2 |
| `{..}`, `{ ..x }`, `{ ..x, -a }` | All records; records of Of(x); forgetting label a from Of(x) |
| `{ x, }`, `{}`, `{ -a }` in a type | Errors: no tuple braces, empty type, or free-standing forgotten label |
| `Int - -1`, `-{ {1} }`, `-{-1}` | Difference excluding negative one; literal negative one; error (IntPart cannot contain a minus) |
| `Int - {1}`, `(1)`, `-(1)` | Difference excluding one; one-part tuple type; error |
| `(x) -> { name: String }` | Function type with an Of parameter and a record result |
| `(x) -> { name: String } { name: x }` in an expression | Function with record result annotation and record body; x in the body is a value expression |
| `{ x: 1 -> 2 }` in an expression | One-clause block, fitted annotation 1; T1 replaces A1’s old example |
| `{ x: y -> 2 }` in an expression | One-clause block with Of(y), not a record |
| `{ f: (x) -> Int { x }, }` in an expression | Record; the explicit comma disambiguates T3’s expression scan |
| `v \|> { x: Int \| y -> 0  _ -> 1 }` | The whole `Int \| y` is a type; after T2 its validity depends on y, so it is not an unconditional syntax error |

The last table row escapes pipes only for Markdown; the i tokens are `|>` and `|`. Record-type parsing does not reuse the expression rule that scans for top-level `->`, `?`, `|`, and `=`: `{ f: (Int) -> String }` in a type is always a record type. T3’s expression printer must retain its required comma even in multiline output.

### Answer to meta.md open question 2

- **(a)** Lowercase head with joined `(` is a macro call in a type, including `a.m(...)`; bare lowercase is T2. No runtime function call in a type and no lookup-based fallback.
- **(b)** A one-Type brace form is a group. `Label : Type`, `..`, and `{..}` are record forms even with one field. Group removal must inspect the node kind, not the number of children. The Lisp/core reader must distinguish a field datum from a grouped Type before removing groups.
- **(c)** T2 P1 decides from the written RHS after groups: an UpName head with arguments and a bare value type RHS is an alias pattern. Freeze this before child expansion. It does not depend on imports. `Codec(User) = codec` is a pattern; `Codec(User) = make_type()` remains a generic alias form, even if expansion later returns `codec`.
- **(d)** `s.Schema(User, _) = user` stays a value binding, as accepted §2.2 and T2 P2 require. Import Schema for an alias pattern. Do not take already valid qualified constructor bindings away.
- **(e)** Bare unqualified UpNames in pattern-part positions bind aliases, with E0537 on imported/built-in collisions. Use qualified forms for exact named tests.
- **(f)** Applied nested heads name existing types; their pattern arguments can bind. No record/literal/function/macro-call pattern parts in T2/T3.
- **(g)** Definitions returns the written item, including alias patterns and Of syntax, with binder spans; it does not demand its inferred body. Types uses only completed allowed results.
- **(h)** Use T2 M3: no generated direct Of in a typed expansion; an existing value-dependent alias needs all same-module sources in strict ancestor groups, or completed imported sources/module-start readiness. No new fixation demands, no dependence on incidental tie order. Run T3 label checks again; no new label parameters after inference.

For `@m(...)`, accepted syntax §2.2 wins: unqualified built-in attribute names are built-ins, regardless of any value named m; their arguments remain positional string literals and no-argument built-ins have no parentheses. Other names, including qualified built-in spellings such as `@a.main()`, are reserved macro attributes. Empty `@m()` is parsed and reserved, not reclassified as `@m`. `@main()` is E0302. An attribute before a macro item remains reserved syntax until M2, not a new T1–T3 feature. The draft’s proposed rejection of empty macro attributes and its older alias-classification rules must be corrected in the macro lane; T1–T3 may not adopt them silently.

Reserve the type-call AST, item-call and attribute-call forms, node-kind-aware groups, alias-pattern classification, provenance, and expansion-safe future table migration. T1–T3 must not execute macros, reserve library names, accept arbitrary expressions in TypeArg, or claim that shortened hover text is Syntax. The future Types.members contract keeps T1 §5.9: expand finite scalar sets into individual literals, keep All/Except as one item, and sort by full show text (`10` before `2`). A record/spread remains one item; this is an API decomposition, not a change to the solver’s meaning of “member”.

## 6. Replacement spec text and section ownership

Apply the individual accepted replacement texts only where the table says to, with these joint replacements taking precedence at overlaps. This is a patch to the combined spec, not three sequential whole-section replacements. Keep all existing section numbers. T1 adds subtype rule 8; T3 adds rule **9**, not another rule 8. New §4.9, §6.13 and §14 are unoccupied in the spec tree read here (`docs/spec.md:319–419,494–555,1031–1084`); later owners must recheck those anchors.

### Text for the shared paragraphs

**§2.4, after the ordinary export rules:**

> A type bound by an alias pattern is an alias with no constructors of its own; exporting it with `(..)` has the same effect as exporting it alone. An exported alias or annotation that depends on a value is checked after its type is complete and must not expose a private nominal type. Its source value need not be exported. Reading from another module never fixes that source’s weak type; a new permanent read requires a wholly closed completed source, or the consumer uses a closed alias exported by the source module (§4.7).

**§2.5, after Order:**

> A type that names a value adds a dependency for type inference, including through aliases and record fields or spread bases. It adds no runtime need or full need. Expression uses still determine recursive groups. A type dependency within such a group, or a cycle between groups after type dependencies are added, is a compile error. A binding with label parameters may not belong to a recursive group with another binding (§4.9.4).

**§3:** replace the entire grammar with §5 above. Retain the accepted arrow, precedence, holes, and declared-variable bullets, and insert:

> A lowercase name or module member without a following joined `(` is the type of that value, under §4.7, not its narrowed known type. A qualified value type names a module member, not a field. A lowercase head followed by `(` is reserved for a macro call. Its arguments are types, including literal types. This classification does not depend on name lookup.
>
> A `{` in a type begins a record type when followed by `..` or by a label and `:`. Otherwise it groups exactly one type. `{..}` is the type of all records; `{}` is not a type. Breaks in record types separate fields; breaks directly in type groups are errors. Literal types are Int and String literals only; a minus at the start of a primary type requires an integer literal, possibly grouped. Thus `Int - -1` is a difference and `-{1}` is the type `-1`.
>
> A value type may not read a name pending in its own pattern or function header, or its binding’s reserved name inside its own value. Distinct inner binders may cover that reserved name after entering scope. Value-dependent types are forbidden in constructor fields, external or host declarations, and service/environment types. A key parameter or a binding with label parameters may not be read as a value type (§4.9.4).

**§4.3:** install T2 §7.4, including syntactic RHS classification, unqualified alias-pattern heads and no record pattern parts. Add:

> A record type may be the closed result bound to a pattern’s alias name. This does not make a record-shaped type pattern legal. Generic aliases may contain ordinary formal parameters and closed value-derived parts, but their formal parameters may not be record labels.

**§4.4, members/places:** install T1’s value-set union/intersection/difference paragraphs and T3’s exact-record-union exception (records §20) together. Add:

> Int and String members merge their value sets. Record union retains all alternatives; it never replaces them by common fields. Record intersection combines all required fields (§4.9). Runtime overlap and head removal count every record or spread alternative as Record. Closed record unions are ordinary types in every position. Open record unions are also legal to write; U3 restricts only inclusion obligations that need alternative-specific bounds. Ordinary variable-member place restrictions remain. Every operation must be valid on every inhabited alternative and returns the union of the results.

**§4.5:** keep rules 1–7 with T1’s amendments to disjointness and environment equality. Rule 8 is the T1 value-set subset rule. Rule 9 is “Records obey the width, intersection, exact union, and forgetting rules of §4.9.3.” There is no literal-subset rule for record labels.

**§4.7:** install T2 §7.5’s full completion rules, retain principal inference/value restriction, and retain T1’s full-versus-limited text rule. Add:

> Simplification and type display must preserve type variables that affect invariant arguments, even when their input and output forms print alike. Reading a type never changes a weak variable into a generalized one. Record fields and the base of forgetting preserve the direction of their surrounding place; invariant arguments inside them remain both directions.
>
> At source completion, ordinary group checks finish before any permanent alias is published. All own-module demands use one snapshot and one checked batch of completion constraints. Record and literal parts are retained exactly. A later or foreign definition read cannot project through a partly open enclosing source, even to a fixed component; use the closed alias made beside that source.

**§4.8, rule 12:** use T1’s entire replacement of the taken-members sentence and append:

> A taken record member or spread member is represented by `{..}`. A record test takes all records; field types do not form runtime cases. This is independent of the finite or cofinite value tests of Int and String.

**§4.8, rule 13:** replace with:

> 13. **Fields and tuple parts.** For `v.f`, when v is known to be made by one constructor (§7.5), or v has a known type whose members are all custom types (including no members), every constructor that may have made v must have field f. The inferred result is the union of those field types with the type arguments from the corresponding bound forms, or the constructor-pattern arguments already supplied by §7.5. The inferred type of v must fit the bound. With no possible constructors, the result is Never.
>
> Otherwise, when v has no known type, or its known type has at least one member and only record, unknown, or variable members, `v.f` has a fresh type A and requires `type(v) <: { f: A }`. Its known type is the guaranteed field type from the conservative record summary, when available. Do not add a constraint against that known field fact.
>
> For `v.k`, v must have a known type and every member must be a tuple with more than k parts. Its inferred type must fit the bound of that known type, and the result is the union of parts k from those bound forms. In every other case field/tuple access is a compile error.
>
> The bound in the constructor and tuple rules takes all Int and String values at covariant places, removes exclusions of only some of those values there, and takes fresh holes at noncovariant places. Record fields and spread bases are covariant for this operation. Use this bound also when a known function type constrains a call. Known field/tuple results are obtained separately from the original known facts (§4.8, known types), not by narrowing the inferred result to them.

**§4.8, known types/arguments:** apply T1’s unknown-part, hidden-input, whole_lits and prefix-rest rules; T2’s inferred-shape paragraph; and T3’s record expected/known fields, with this joint addition:

> A value type in an annotation reads its source’s inferred type. A local unresolved variable supplies no known shape from its temporary bounds. Known summaries of record spreads keep only guaranteed record fields. Missing known fields supply unknown information, never proof of no possible value. Record fields and spread bases are visited by the same hidden-input and whole-literal rules as other covariant parts. Label parameters are substituted in both the inferred and known callee type before matching other arguments. Checking key arguments first does not change expression evaluation order.

**§4.9.2–3, forgetting:** apply T3’s set meaning and rules, with the J8 qualification:

> Forgetting from an empty type is empty. Dropping a field from a record type is an exact rewrite only when the discarded field is known to be inhabited for every remaining choice of its variables. Otherwise the forgetting form remains. Closing a value-derived type may make this rewrite possible later. Normalization before or after substituting a closed type must describe the same set. Normalize a record with a plainly empty field to Never. Preserve exact union alternatives before and after substitution, including label substitution. Common-field joins belong only to conservative known summaries. Forgetting distributes over exact union, not over intersection.

**§4.9.4:** retain T3 L1–L9, and replace its value-type sentence with:

> A value type or alias-pattern source may not name a key parameter or a binding with label parameters, even though ordinary value types read inferred schemes. A known singleton key selects a label; the inferred type of that argument must then fit that singleton. Narrowing a new clause binder can supply such a type. A narrowed known fact alone does not.

**§7.1–4:** combine T1 literal tests with T3 record tests and T2 conversion. Insert:

> A type test first reads any allowed value type or completed alias. Int and String members test the value set as well as the head. Record and spread members test only the record head, and every possible record at that place must satisfy their written field requirements. A type with an unresolved variable member cannot be tested. There are no record patterns. A fitted one-clause block, including choices, may report to Assert; it is not checked as an exhaustive multi-clause block.

**§10.3:** keep existing host forms and T1’s directional paragraph; append T3’s prohibition in both directions:

> Records may neither be given to the host nor received from it through this interface. A binding with label parameters cannot be an entry point even when its ordinary parameter types would fit the host interface. Value-dependent written types in early host declarations are rejected before these form checks (§3).

### Complete section edit map

| Spec section | T1 | T2 | T3 / combined gate |
|---|---|---|---|
| §0, §2.1, §12 | — | — | Add the ordinary record module and §14 pointer; IO owns the final module count |
| §1.7 | Negative/group behavior respects existing line rules | No independent line rule | Record-type/expression breaks as separators; preserve type-group errors |
| §2.2 | Preserve macro/attribute reservations | Freeze alias-pattern classification without changing qualified bindings | Preserve header and item decisions |
| §2.4 | Literal API types remain exact | Completed alias exports/private types | Structured record/label identity; joint paragraph above |
| §2.5 | Grouped negative literal directness | Type-dependency order | Record expression uses/direct literals and label SCC restriction |
| §2.7 | Full literal interface comparison; no compatibility-policy change | Private source changes can affect public aliases | Label alpha-renaming and sorted fields; no new package identity rule |
| §3 | Literals and TypeArg=Type | ValueType | RecordType; use one grammar |
| §4.1–2 | Value sets/literal types | No new value set | Records are a new domain; fields are types, not ValueSet entries |
| §4.3 | Preserve literal leaves in ordinary aliases | Alias patterns/closed bodies | Records in alias bodies, no record alias-pattern parts |
| §4.4–5 | Set algebra, rule 8 | Expanded-place validation | Spread exception and rule 9; J8 |
| §4.7 | No widening/full text | Completion and value restriction | Shown-place checks; preserve K1 |
| §4.8 | Rules 2, 11–13; fitted literals; deferred Assert; known calls | Pattern annotations and inferred-shape Of | Rules 13a–c and 15; record known/expected parts; joint replacements above |
| §4.9 | — | Completion references this section | Add T3 records/labels, corrected J8 |
| §5 | Preserve declared UpName scope | Alias binders and pending value scope | Label references use declared-variable identity |
| §6.1 | Literal-annotation clause example | Of/macro annotation examples are not syntax errors | Record scan and constructor-update naming |
| §6.2 | No change to runtime order | Type reads do not run values | Record fields/spread order; key inference cannot reorder effects |
| §6.3, §6.5, §6.9 | Literal types, list example, operator results/E0520 | Of never constant-folds a value | Same-head records are not compared by nested literal disjointness |
| §6.7–8 | Keep widened known bound | — | Record access/body and constructor-update terminology |
| §6.10 | Negative literals through groups | Read types preserve generalization restriction | Direct record literals; spreads remain non-direct |
| §6.12 | Implement fitted choices, remove A1 temporary restriction | Of scope in patterns | No new record-pattern syntax |
| §6.13 | — | — | Add record expressions |
| §7.1–4 | Literal/cofinite tests, usefulness, witnesses, Assert | Expand Of before test validity | Record/All; preserve scalar coverage; joint paragraph |
| §7.5 | No new literal narrowing of original subject | Of does not read known constructor flow facts | Distinguish new binders from original subject |
| §9.2 | Literal arguments obey service equality | Forbid value-dependent environment syntax | Record is not a service; rename old “record of functions” |
| §10.1–3 | Directional sets with existing codes | Early restrictions and deferred export annotation checks | Records prohibited; no label entry points |
| §11.1, §11.4, §11.8 | Canonical literal equality/escaping stays | No runtime change for reads | Add record values/equality/text; one escape implementation |
| §14 (new) | Supplies singleton String keys | No library privilege | Ordinary `i/record` declarations/operations from T3 |
| Future macro sections | Literal Syntax and members decomposition | Written Definitions and readiness | Record node kinds, hygienic label parameters; §5 contract |

### Record substitution amendment (T3 hand-off)

Records §20 replaces the common-field union and one-Rec routing assumptions in J2, J4, J5 and J8; J1, J3, J6 and J7 retain their contracts. This section takes precedence over the earlier representation sketch and routing list. Use N1/N2, E0–E5 with J8’s inhabitance guard, and U1–U6. Shared Union/Inter/Without nodes retain exact alternatives without eager expansion. `head()` identifies a runtime constructor, not permission to merge alternatives. A known summary is not a solver type or a permanent alias. Do not join field bounds merely because two lower record bounds share a label.

Install the revised record paragraphs in spec §4.4, subtype rule 9, and §4.9.2–3 as in records §20. Right-side multiple-record constraints must use the no-new-bounds gate U3 before left-intersection or Without routing. Remove the record-count/spread output-only restriction from written and inferred types. Keep existing ordinary-variable place restrictions. U3 includes pure structural field inclusion, exact field case splitting and read-only established bounds; retry unresolved obligations after ordinary group constraints and before generalization/T2 publication. Never read a lower bound as an upper bound or create a branch-specific bound. All alternatives feed field checking, type tests and coverage; only runtime Record/All tests are deduplicated. Empty normalization can remove a record from coverage, but known summaries may remain conservative.

This amendment proves normalization commutation for the nonrecursive record fragment. It does not import MLstruct’s principality theorem into i’s different set semantics. T3 must test the choice-free inference restriction and preserve J7; unproved open right-union constraints report the missing inclusion, and actual bound choices report the choice. Neither is solved by selecting field bounds. Use the resource diagnostic, not either proof diagnostic, when U6 exhausts its 65,536-state demand budget. Required tests are in records §20, `tests/t3-record-substitution.py` and `tests/t3-record-demand.py`.

R1–R4 acceptance hand-off: install spec §4.9.3's structural/bound proof, every-position closed-union, graph/interface and resource paragraphs together. Accept `wrap_union`, `pair`, the branch-derived `Schema(Row)` input and closed `forward`; test both directions of established bounds and reject choice-dependent fresh variables. U6 replaces eager DNF construction throughout J4 completion, J7 rebuilding, display and interfaces. Its graph bundle refines full-source read-back for shared record types; small types still use source. No common summary may become a published alias. Require the 20-step fresh-label branch and Fan scale gates, including graph round-trip and Fan(Never), well under one second excluding compiler startup. Earlier J4/J5 tables and later test lists that expect E0518 solely for record/spread unions are superseded by this section. Current source locations and exact changed expectations are in records §20.6; full principality and arbitrary leaf-algebra completeness are not claimed by the model.

Prior-art algorithm hand-off: records §21 selects Elixir’s lazy BDD and map emptiness/projection as the demanded proof kernel, with shared suspended operations for i’s open/undemanded types. Use its reference-first implementation table; do not replace it with MLstruct’s common-field union or eager DNF products. Unknown and ResourceLimit are distinct from Empty and Refuted. Pending U3 obligations retain their owner, scope and origin through ordinary solving and T2 completion, and are discharged or diagnosed before generalization/publication. Preserve one scheme-wide variable map across branches and invariant occurrences, including retained ordinary constraints and graph-interface metadata. Add `tests/t3-record-bdd.py` to the two existing models, the negative-map projection, cache-off, reordered-atom and resource-retry gates, and the demanded closed-Fan scale probe. These refinements change no example verdict from the R1–R4 contract.

## 7. Error allocation and reporting order

No track claims a code from A1’s reservations. Retiring a reservation does not free its number for another error. Current checker codes end at E0522 (`crates/ilang-check/src/lib.rs:74–96`); A1’s approved list is `a1-front-end.md:245`, confirmed by the ledger. The combined allocation is:

| Code | Owner and meaning |
|---|---|
| E0113 | A1 DualAmbiguous; retired by A3 |
| E0114 | A1 RecordReserved; T3 removes the reservation, no reassignment |
| E0115 | A1 HeaderGroup; retained |
| E0229 | A1 value-type reservation; removed by T2 |
| E0230 | A1 type variable with arguments; retained |
| E0231 | A1 type-variable/type-name collision; retained |
| E0232 | A1 type parameter form; retained for generic heads |
| E0233 | A1 alias-pattern reservation; removed by T2 |
| E0234 | A1 macro reservation; T1–T3 retain it |
| E0235 | A1 one-clause choices temporary error; removed by T1 |
| E0236 | A1 mixed old/new variables; retired by A3 |
| E0523–E0529 | Reserved to T1; no new numbered error is needed by the accepted T1 rules. Keep unused. Float literal type is an existing syntax diagnostic. |
| E0530 | T2 VALUE_TYPE_CIRCLE |
| E0531 | T2 VALUE_TYPE_PLACE: early/trusted or forbidden typed-expansion read |
| E0532 | T2 TYPE_PATTERN_FORM |
| E0533 | T2 VALUE_TYPE_OPEN: used general/open definition part |
| E0534 | T2 VALUE_TYPE_NOT_FIXED: weak closure, open completed source, or failed trial |
| E0535 | T2 TYPE_PATTERN_MISMATCH |
| E0536 | T2 VALUE_TYPE_OWN_SCOPE |
| E0537 | T2 TYPE_PATTERN_BINDER_IS_TYPE |
| E0538–E0539 | Unallocated; do not consume without coordination |
| E0540 | T3 DUPLICATE_LABEL |
| E0541 | T3 LABEL_NOT_KNOWN, inconsistent repeated key, invalid recursive key |
| E0542 | T3 LABEL_PARAM_VALUE |
| E0543 | T3 LABEL_PARAM_PLACE, including key Of and multi-binding recursive group |
| E0544 | T3 LABEL_MAY_BE |
| E0545 | T3 LABEL_PARAM_ESCAPES |
| E0546–E0549 | Reserved to T3, currently unused |

Reused codes: E0201/2 unknown value/type; E0203/4 member/export; E0205 module as value; E0207 value as module; E0208 duplicate definition; E0214 import cycle; E0221/2/6 parameter use/scope/duplicates; E0223 alias cycle; E0224 arity; E0225 service head; E0227 hidden type; E0301 reserved macro attribute; E0302 built-in attribute arguments; E0303 placement; E0308 declaration reservation; E0318/E0320/E0521 host/entry types. E0501 remains mismatch (including Values outside the precise deferred case and later conflicts with fixation), E0503 invariant equality, E0504 ambiguous routing, E0505 occurs, E0506 missing known function shape, E0507 argument labels, E0509 top effects, E0511 variable test, E0512 field failure, E0513 variable subtraction, E0515 service equality, E0516 coverage, E0518 places, E0520 disjoint equality, E0522 entry environments. W0501/W0502 remain usefulness/unused-value warnings.

Precedence is local, not a demand for one error in every invalid program: parse/classification errors before resolution; pending names before lookup; Of provenance bans before early shape/ABI checks; R7’s whole-source gate before projection; label-source bans before attempting to close a forbidden source; candidate/shape failures report E0534/E0535 without leaked trial failures. Ordinary later constraints retain their normal code plus fixation provenance. T1 alone handles Values-versus-E0516 suppression as J5 describes.

## 8. Joint tests and expected results

These are implementation acceptance tests, not claims that this pre-A1 compiler accepts the new syntax. Standalone files must supply their ordinary imports/helpers. Expected values follow the named rules. Compile-error cases must not be run as if they compiled. Runtime cases run at O0 and O2.

### Programs at the important boundaries

**T-A: literal alias through a record (J1, T2 R2, T3 width).**

```i
r = ( method: "GET", port: 8080 )
Config = r
use = (c: Config) { c.port }
value = use(r)
```

Config is `{ method: "GET", port: 8080 }`, value is 8080. `use({ method: "POST", port: 8080 })` fails E0501. A separate source annotated `{ method: String, port: Int }` yields that wider alias. This detects Of-to-known confusion, literal widening, and incorrect package serialization.

**T-B: fitted literal read (J1/J2).** Use `copy` in §3. `copy("a")` gives `("a", "b")`, no false E0501; `copy("b")` fails through Assert. The bad key use in J1 is E0501, not successful lookup and not E0541. Add nominal `Box(A) { Box(value: A) }` and `(Box(r), 0).0.value.value`; reading it inside copy still gives `"a"` without a false inferred String-to-singleton constraint. Bind that result to p and then write `q: p = "b"`: q is accepted and gives `"b"`, proving that the inferred result also came from the bound forms.

**T-C: legitimate narrowed key (J1/J5).**

```i
read = (s: String) {
  s |> {
    other: String - "a" -> 0
    key -> {
      same: key = "a"
      record.get(( a: 7 ), same)
    }
  }
}
```

It gives 7 for `read("a")` and 0 for `read("b")`; key/same have singleton type `"a"`. A separate negative fixture inserts `Key = key` inside the block: blocks have no type definitions (T2 P5/spec §4.3). A companion alias test binds a closed value at top level. No local alias feature is added to make a test pass.

**T-D: keys remain identities (J1/J3).**

```i
forward(K) = (r: ( K: Int ), key: K) -> Int {
  record.get(r, key)
}
answer = forward(( a: 7 ), "a")
```

Value 7. Changing the body to `copy = key` then `record.get(r, copy)` is E0541. Adding `x: key = "a"` in the body is E0543. `Get = record.get` is E0542. Two parameters of the same K passed distinct fixed strings or distinct outer keys give E0541. A singleton expression that records an effect before returning `"a"` runs that effect exactly once, in written argument order, when used as a key; test both a labelled call and a pipe.

**T-E: records do not acquire runtime field tests (J5).**

```i
bad = (r: { a: Int | String }) {
  r |> {
    x: { a: Int } -> x.a + 1
    _ -> 0
  }
}
```

E0501, even with the catch-all: the first test takes all records. The positive version uses `r: Int | { a: Int }` and `n -> n + 1` as its second clause; it returns the field plus one for a record and the integer plus one otherwise. The second binder is Int. Replacing the record annotation by a completed alias changes neither the test table nor the result. Combine a record clause with `s: String - "a"` and a remaining `"a"` clause; test with an Int rejection where the declared input excludes Int, and with a declared Int branch where it includes it. The string complement must still have its tag guard.

**T-F: invariant generalization (J7).**

```i
Cell(A) { Cell(run: (A) -> A) }
make = () { Cell((x) { x }) }
int_cell: Cell(Int) = make()
string_cell: Cell(String) = make()
nested = () { ( cell: Cell((x) { x }) ) }
```

Both calls are accepted with distinct instances. `nested` keeps a general variable in the invariant child of its covariant record field. `Same = make` is E0533, not a closed alias to `Cell(Never)`. `wrong = (c: Cell(Int)) -> Cell(Never) { c }` fails invariant checking. Also run K1’s service-equality, contravariant, bivariant and stream fixtures after migration to the accepted syntax; do not replace them with a check only of printed variable names.

**T-G: completion fixes constraints, not display (J4).**

```i
Cell(A) { Cell(get: () -> A, set: (A) -> Nil) }
make_cell(A) = (x: A) { Cell(get: () { x }, set: (y: A) { Nil }) }
r = make_cell(1)
Read = r
later = () { r.set(2) }
```

Read fixes the demanded invariant argument to 1 if the completion batch succeeds; later fails with the normal type error and fixation note. Annotating the source `r: Cell(Int)` permits later. Reading through an ordinary local cannot clone the weak variable. A deliberately failing trial must leave solver state unchanged, including T1 fused bounds and T3 record/label propagation. This is checked by an internal before/after state test, followed by an unrelated valid demand in recovery mode.

**T-H: codec and consumers (J4).** Compile T3 §5.9’s exact two-parameter builder with T2 M1a’s module placement. User is `{ age: Int, name: String }`; decode produces a value usable by a function annotated User; later encode uses still constrain the distinct encode variable. A consumer’s `Codec(Remote, _) = codecs.user` is E0534 if encode remains open. Importing `User` and writing `Remote = User` succeeds without changing source equations or WAT. `Codec(_, Enc) = user` in the source fails E0534 if Enc is unfilled. Repeat with all source parts explicitly closed: foreign extraction then succeeds. No schema-specific solver branch is allowed.

**T-I: exact forgetting (J8).**

```i
Gone(A) = ( ..( a: A, b: Int ), -a )
empty = (x: Gone(Never)) { x |> {} }
```

`Gone(Never)` is Never. In the internal normalization test, normalize Gone(A), then substitute Never: it must also become Never. Replacing A with 1 yields `{ b: Int }`. Add an invariant wrapper around Gone(A), and test both polarities. This catches an erased variable even if no runtime value can inhabit its original record. It also ensures T2 cannot cache the result of an inexact open rewrite.

### Additional matrix

| Test | Expected result and governing rule |
|---|---|
| Full grammar table in §5, both parse and print round trips | Unique tree or the specified diagnostic; changing imports cannot change TypeCall or alias-pattern classification |
| `Port = 0x10`, then same interface with `16` | No interface change; T1 canonical Int |
| Alias of 20 literal strings, change only last string | Interface changes, despite limited hover; §4 |
| `{ "a": 1, "\u{61}": 2 }` and analogous type | E0540; decoded labels |
| `f(K,J)` renamed consistently to `f(Z,A)` | Identical canonical interface after renaming before sorting; §4 |
| Fixed label `"K"` beside parameter label K | Remain distinguishable in read-back and canonical text; §4 |
| `f(A) = (x: A) { x.name }` | Principal structural field constraint, no rigid ordinary variable; J4 |
| `list.fold` with `{ n: 0 }` and a full-literal callback comparing `acc.n == 3` | No false E0520; T1 hidden inputs extended to fields, J2 |
| Same fold updates with an unannotated spread | E0505; T3 occurs rule, not a hidden-input regression |
| Record-field union beside a retained spread in an input type read via Of | E0518; T2 whole-expanded annotation and T3 shown-place check |
| `a = (n: b) { n }` and `b = a(1)` | E0530, with chain; type edges do not merge SCCs |
| Alias dependency in `{ nested: List(source) }` | Orders source completion before reader; shared visitor reaches fields |
| Alias-only importing module with no bindings | Module-start completion/export checks still run; J4 |
| Private nominal type exposed by a completed record alias | E0227; deferred whole-body export check |
| Host external with return type `"a"` | E0318; host cannot supply restricted set |
| Host external with a value-dependent alias of Int | E0531 before ABI checks; J6 |
| Record at either host boundary direction | Existing host type error; no record ABI, J6 |
| Label-generic host export with only String ABI slots | E0320; L9 |
| Complement string test on small/boxed Int values | Never enters string branch; T1 tag-and-equality lowering |
| Record `Without` with unknown base compared with Int | Disjoint record domain, no falsely lost record possibility; J2/J5 |
| `list.map([], (x) { x.name })` | Builds/runs without lowering panic; empty Field access list trap is unreachable, T3 D6 |
| Independent group tie order reversed | Same completed aliases and diagnostics; T2 snapshot ownership |
| Non-generalized binding’s shown spread input union | E0518 even without alias demands; ledger/T3 place check |
| One clause with choices versus two clauses | Former fitted/Assert, latter coverage; A1 gate and J5 |

For type-only changes, compare WAT under the existing dump policy. T2 demands do not add value evaluation; changing actual inferred type tests can legitimately change code, and such differences need named cases. T3 changes record runtime sections and record-using code; existing non-record generated bodies retain their behavior. Runtime representation/equality/hash/inspect and long-label tests remain T3 §15’s gates; this note does not replace them.

## 9. Ordered track checklists

`[shared]` means the file/region must retain earlier contributions from §2.3. T1 and T2 can be developed side by side; T2 integrates only after T1. T3 **starts after T1 lands**, can work on independent record/runtime tests while T2 finishes, and rebases/merges shared work only after T2. This corrects T3’s overly strict “starts after T2” sentence without allowing out-of-order shared-file merges.

### T1

1. Map baseline anchors after K1, A1/A2/A3; retain K1’s tests and A2’s Namer. Install T1 spec edits with joint §3/§4.5/§4.8 wording `[shared]`. Gate: source/code audit and reserved-form parse fixtures; G3 approval remains the coordinator’s implementation gate.
2. Add pure ValueSet algebra/canonical literals; add the Con field and TestMember table `[shared: mod, solve, display, known, infer, coverage, pattern, lower]`. Gate: model/unit algebra tests and no changed corpus types/WAT while all sets are All. Audit wildcard Con matches and head comparisons, not merely compiler exhaustiveness errors.
3. Add solver/display/coverage machinery without enabling literal behavior `[shared]`, including full interface text `[shared: package, query, shown]`. Gate: T1’s staged unchanged-output checks and K1 invariance regressions.
4. Fix deferred group may-fail before generalization `[shared: infer, pattern, resolve/lower as needed]`. Gate: T1 group/self-holder/local-function tests at O0/O2, Assert in the correct effect frame, E0501 at the binding.
5. Enable literal expressions, fitted-versus-match patterns, known-bound and hidden-input fixes, scalar witnesses, inferred host-export checks `[shared]`. Gate: T1 step 7 corpus count, all changed goldens read, no false E0520 in the accepted counterexamples. Keep the no-widening rule.
6. Add written literals/negative groups, TypeArg=Type, HIR Lit and every visitor/writer arm; enable set tests and rule 12 candidates `[shared: syntax, hir, resolve, query, package, infer, pattern, lower]`. Gate: grammar table subset, finite/cofinite coverage and boxed Int test runs; no dropped Values failure.
7. Implement fitted choices for one clause, replacing E0235 `[shared: resolve, hir/pattern, infer, lower]`. Gate: `tests/spec/6/one_clause_choices.i`, Assert and non-generalization equivalence, no §7.5 subject-constructor leak. This is required even though T1’s older numbered checklist omitted it.
8. Full T1 tests, existing suite and K1 regressions; send measurement script/results to coordinator, who measures quiet-window performance. Gate: T1 RF and approved E0520 changes. T2 rebases on this result; T3 may start.

### T2

1. Develop against A1/A3 while T1 proceeds, using the agreed Lit/Con contract; rebase on landed T1 before integration `[shared: every row of §2.3 touched by T2]`. Gate: all T1 and K1 regressions preserved, no duplicate TypeKind or code allocations.
2. Install T2 spec edits/alias classification; add Of/sites/pattern metadata, identity scopes, provenance and complete type visitor `[shared: syntax, hir, resolve]`. Gate: syntax/import-independent classification, own-scope, cycles and early-context errors.
3. Add type-dependency graph over expression SCCs and module-start/source-completion events `[shared: order, infer]`. Gate: runtime needs/init unchanged for type-only references; reverse tie-order test; alias-only module test.
4. Implement snapshot closure and full solver trial/rollback `[shared: solve, infer, display]`. Gate: pure-variable SCCs, reverse links, invariant variables, earlier-open-variable rejection, failed-trial state equality. T1 bound fusion must remain observed; no replacement-variable semantics.
5. Wire ordinary annotation instances/local sharing, exact permanent reads, R7 whole-source gate, safe known-shape conversion, and whole-expanded place checks `[shared: infer, known, pattern]`. Gate: T-B/T-F/T-G, no local alias escape, no hidden Of under union/intersection.
6. Complete aliases eagerly and defer exports correctly; publish full alias_types and query/interface consumers `[shared: mod, shown, query, package]`. Gate: private nominal leaks rejected, private source rename stable, >12 literal change detected, failed/Pending aliases never serialized as holes.
7. Run T2 suite with T1 enabled, reverse tie order and error recovery; K1 tests and ordinary corpus checks. Gate: full verification plus T2 RF; no WAT evaluation from type dependencies. T2’s scalar Codec fixture proves scheduling; the actual record builder remains T3’s gate.
8. Hand T3 one closure extension point, fixed-form/provenance visitor, reusable context checker and resolved Of references. Gate: no partial-shape API promised to foreign readers; M4 readiness tests documented, not prematurely enabled.

### T3

1. Start after landed T1; implement independent tests/runtime work, then rebase shared edits onto T2. Adopt J8 or obtain an alternative sound coordinator decision before implementing E4. Gate: shared-file and spec region map includes T2’s trial state, not only T1’s Con field.
2. Add Label/Rec/Without/smart constructors and record solver rules `[shared: mod, solve, display]`. Gate: per-site Con/Fn/head audit, width/spread/routing/occurs tests, J8 substitution test, K1 invariant child test; no catch-all that loses bounds.
3. Add exact full display, structured shown labels/key-slot metadata, known summaries, hidden-input and inferred-shape traversal `[shared: shown, display, known, T2 closure module]`. Gate: read-back, label alpha-renaming before sorting, no foreign Param in expected types, no false E0520 in full-literal fold.
4. Add record syntax/HIR and reusable label/context checks `[shared: syntax, hir, resolve, order]`. Gate: grammar table, comma printer round-trip, decoded duplicate labels, L2a/L5 Of rejection, expression SCC restriction. Add record base/field dependencies and runtime-use walking.
5. Add record inference/access and call preparation `[shared: infer, pattern, coverage, mod]`. Gate: key inferred once, singleton subtype check, repeated-key and recursive-key tests, effect order, Record/All coverage/test tables, shown-place checks on weak bindings.
6. Extend T2 closure, provenance and exports with records/Without `[shared]`. Gate: exact Schema and two-parameter Codec builders, closed own-module alias and rejected foreign partial extraction, failed-batch rollback including record constraints and label escape state.
7. Add runtime/IR/lowering and ordinary record module; preserve T1 TestValues `[shared: lower, library registration]`. Gate: tag tests and complements, empty nominal access, label constants/effect preservation, Wasm validation and O0/O2 equality/hash/inspect tests including size limits.
8. Finish queries and package identity `[shared: query, package]`. Gate: source navigation, structural alias hover, unchanged interface on field/parameter renaming, changed interface on field/literal meaning changes. Full T3/T2/T1/K1 suite and record RF; coordinator measures runtime costs.

Global merge order in these shared regions is T1 → T2 → T3, even where an older note omitted T2’s solver/display work or allowed T2 to land first. Preserve external track ordering: K1 → A2 → T1 for display prerequisites; A3 before T2 resolution; T1 before the later R2/IO/M2 host-boundary edits; T3’s host paragraph after whatever has landed there, with later owners retaining it; M2 after all three. M1’s reader work must take the same grammar/classification contract regardless of whether its code lands before a type track. Do not reorder unrelated C/R2 lowering work to force a single whole-file chain.

## 10. Changes to make in the accepted notes

These are instructions for the owning tracks after acceptance, not edits made by this lane. “Old” quotes a sentence or the identified declaration; “new” replaces that text. The rest of each section stays unless the shared contract above explicitly changes it.

### Changes to make in `literal-types.md`

1. **§5.7, package text.** Old: “`.head()` (:1165) maps `Lit` to the `Int` or `String` head, and `fn ty` (:1351) prints `hir::TypeKind::Lit`.” New: “`TypeShow::head` prints a whole type definition and delegates its alias body to `TypeShow::ty`; add written Lit printing there and full literal printing to the inferred ShownType path. Never replace an alias's literal by its Int/String head.” Reason: the named method is not `Ty::head`; the actual delegation is `package.rs:1316–1323`, and the export call is `:1162–1167`. T2 then supplies completed alias bodies to that path.
2. **§6, replacement §7.1.** Old: “So `v |> { p: (Int, Int) -> 0  _ -> 1 }` and `v |> { x: Int | y -> a  _ -> b }` are compile errors, in the first clause and in any other.” New: “The ungrouped tuple-annotation example remains a compile error. In `v |> { x: Int | y -> a  _ -> b }`, `Int | y` is a type; before T2 y is reserved, and after T2 it reads a value type and is checked by the ordinary type-test rules.” Reason: accepted syntax and T2; the bar inside the annotation is not a pattern-choice separator.
3. **§12, step 6a.** Old: “this includes a one-clause match, which A1 lowers to a binding, and a one-clause block with choices if a marked `case` node exists for it.” New: “This includes a one-clause match and one-clause choices; T1 implements the latter as fitted binding behavior, removes E0235, and must pass `tests/spec/6/one_clause_choices.i`.” Reason: the A1 ledger decision is binding, not conditional on a helper node already existing. Preserve the rest of the deferred-Assert algorithm.
4. **§13, question 9, merge-order sentence.** Old: “T1's arms land in step 8, after T2's `query.rs` and `hir.rs` changes if T2 is merged by then.” New: “T1 lands first; T2 develops alongside it and rebases these shared arms onto T1; T3 follows T2 at integration.” Reason: the assigned joint schedule. The shared-file permission list is §2.3 of this note, not separate whole-file ownership.

### Changes to make in `value-types.md`

1. **§5, M1.** Old: “Normalize `Without(R, {age}) & { age: Int }` after closing its dependencies; T3's exact rewrites yield `{ age: Int, name: String }`.” New: “Close dependencies before normalizing `Without(R, {age}) & { age: Int }`; apply the inhabited-discard condition of types-joint J8. The concrete builder still yields `{ age: Int, name: String }`; an unresolved or possibly empty discarded field keeps its Without form.” Reason: T2 must not permanently cache T3's inexact open-variable E4 result.
2. **§5, record extension boundary.** Old: “Until T3 adds those forms, unsupported closure is an explicit failure, never a partial type.” New: “Until T3 adds those forms, unsupported closure is an explicit failure. T3 must extend closure, provenance, inferred-shape conversion, transactions and full alias display together, and pass the two-module record Codec tests before integration.” Reason: a parser/solver-only record addition does not implement value types of records; make the cross-track gate explicit.
3. **§9, step 1.** Old: “Rebase on A1/A3 and T1; map the cited regions to the new code.” New: “Develop alongside T1 against the joint contracts; rebase on landed A1/A3 and T1 before integration, preserving K1 in every generalization/display path.” Reason: distinguish the start gate from the merge gate and include the invariant fix.
4. **§9, step 10.** Old: “T3 adds record/Without closure and shape arms, label checks, shown-place checks and the §5.9 builder test.” New: “T3 adds those arms, the joint conservative known summaries, exact forgetting gate, structured shown labels and scheme key slots, then tests the §5.9 builder under both own-module fixation and R7 closed-only foreign reads.” Reason: the interfaces needed by T1–T3 are wider than a new closure match arm. This does not weaken R7.

### Changes to make in `records.md`

1. **Status paragraph.** Old: “T3 starts after A1 (the new parser) and T1 (literal types) have landed, and after T2 has merged.” New: “T3 starts after A1 and T1 land; shared work rebases onto T2 and merges after T2.” Reason: the assigned schedule permits independent T3 work while T2 finishes.
2. **§4.3, E4 and its explanation.** Old: “A field type with a type variable is not plainly empty, so E4 applies to it; a variable stands for the types that reach it, and when none reach it, no record of `{ fs }` is ever made, as for `(A, Int)` today.” New: “After E1 and the label checks, E4 may discard only fields whose types are known to be inhabited under every remaining substitution. If that is not established, retain Without; a later closed substitution may enable E1 or E4.” Reason: `Gone(A)` in T-I proves the open rewrite is not an equality. Apply this qualification also to E4's rule line and to §12/§4.9.3's final claim that every known-label record base reduces to a record. No nonemptiness solver is introduced.
3. **§4.7, shown representation.** Old: `ShownType::Record { base: Option<Box<ShownType>>, fields: Vec<(String, Option<ShownType>)> }`. New: “Use the same base/parts form with a structured shown label distinguishing decoded fixed strings from label parameters. Schemes also retain ordered label-parameter identities and key-slot identities.” Reason: a String cannot tell fixed `"K"` from parameter K; ordinary body String types cannot reconstruct a scheme's `key: K` positions. See §4 of this note.
4. **§4.8, members.** Old: “A `Without(B, L)` gives the members of `B`, where a record member loses every field whose label is the same as, or may be equal to, a label of `L`.” New: “For Without, keep only record-domain alternatives, forget equal or possibly equal labels, and use all-records for an unknown base. Join the guarantees of multiple record alternatives before selecting a field; preserve other alternatives of the enclosing union.” Reason: Without cannot contain Int, and a Rec beside a spread is not the only possible record. This is conservative known information, not exact E4 normalization.
5. **§4.8, matching.** Old: “A field with no field of the same label is skipped.” New: “A missing known field is matched with unknown information, including Top for variables in that expected field, under T1's unknown-part rule.” Reason: skipping can leave a variable determined by only another occurrence, producing false E0520; J2 applies T1's fix recursively.
6. **§5.9, alias stability.** Old: “So `Codec(User, _) = user` binds `User` to `{ age: Int, name: String }`, whatever the rest of the module does.” New: “When the own-module completion batch succeeds, User is `{ name: String, age: Int }`; later uses must respect its checked fixation equations. Distinct open encode variables stay live. A foreign partial extraction is E0534; import User, or first close the whole source.” Reason: T2 M1/R7 and ledger G2(S6), not unconditional shape freezing.
7. **§6 and §12, rule 13a/13c.** Old: “the type of `v` must be a subtype of its known type.” New: “the inferred type of v must be a subtype of the bound of its known type, with T1's covariant whole-literal bound and fresh holes at noncovariant places.” Reason: preserve T1's fitted-literal fix. Use the full joint replacement for rule 13 in §6 above; rule 13b keeps only its structural field constraint.
8. **§8.2, grammar.** Old: `Prim       = FnType | TupleType | NamedType | "_" | "{" Type "}" | RecordType .` New: “Use types-joint §5's complete grammar, including Literal, ValueType, TypeCall and TypeArg=Type.” Reason: the displayed pre-A1 grammar would remove T1, T2 and macro reservations. Keep T3's record lines and place restrictions.
9. **§8.3, formatter.** Old: “a record on one line when it fits, `{ name: "ada", age: 36 }`, and else one field per line with no commas.” New: “Use one field per line when needed, retaining a comma after the first field whenever §8.1's scan requires it.” Reason: §8.1's exact-tree rule wins over this summary; a function-valued first field otherwise becomes a clause block.
10. **§11, allocation.** Old: “T1 and T2 number their new codes from E0523 upward and stay below E0540.” New: “T1 reserves E0523–E0529, T2 owns E0530–E0537, E0538–E0539 remain unallocated, and T3 owns E0540–E0549. Preserve A1 E0113–E0115 and E0229–E0236.” Reason: accepted T2 allocation and A1 ledger; see the combined table.
11. **§11, E0541 hint.** Old: “A clause that matches the key does not help”. New: “Matching does not turn the original subject name into a singleton-typed key. A new binder whose inferred type is a singleton after T1 rule 12 can be a key; a fitted binder with only a singleton known fact still fails the inferred subtype check.” Reason: T-C versus T-B. The existing example using the original s remains rejected.
12. **§12, §4.5 addition.** Old: “Add after rule 7: "8. Records: see §4.9.3."” New: “Add after rule 8: 9. Records: see §4.9.3. T1 owns rule 8.” Reason: T1's value-set subset rule is already present. No other rules are renumbered.
13. **§14, package canonicalization.** Old: “It renames the `Param` labels of a `ShownType::Record` the same way, in the order of the label parameters of the scheme, to `'k0`, `'k1`, …, so that renaming a label parameter is no change either.” New: “Retain that ordered parameter metadata; rename structured labels and key slots by it, sort by canonical label identity, and only then traverse record fields to name ordinary variables. Use the completed alias body for value-dependent aliases.” Reason: correct alpha-equivalence must not depend on source label spelling or source field order; T2 changes the alias data source.
14. **§16, steps 6/9 and final gate.** Old: “Then accept the changed outputs with `UPDATE=1` and read each change.” New: “Before accepting changed outputs, pass the joint rule-13, missing-known-field, Record/All coverage, key evaluation order, K1, exact-forgetting, and T2 record-completion tests. Then read every changed output.” Reason: individual T3 tests do not cover the new shared boundaries. Retain its runtime and scale gates.

Two corrections are handoffs to other owners: A1's `TypeArg` enum collapses to Type when T1 lands, and the macro draft must take §5's answers, especially syntactic alias classification, qualified constructor bindings, and empty attribute calls. Do not edit accepted syntax to match the older macro draft. The built-in attribute distinction is a syntactic attribute rule, not a privileged library name.

## 11. Self-attack and verification record

These are my own reviews, not independent RF. New-language program outcomes below were derived from the accepted rules and the cited code. They were not run through an implementation of T1–T3.

### Pass 1: attack the initial reconciliation

| Attack | Concrete counterexample | Finding and repair |
|---|---|---|
| Soundness: field tests | T-E, followed by a call with `{ a: "s" }` | A field test must not select the Int records. Keep Record/All and the inferred field constraint; do not hide its mismatch as a scalar Values failure. |
| Soundness: flow facts versus types | T-B, and `record.get({ a: 1 }, x)` after `x = "a" = s` | T3's raw rule-13 bound reintroduces false E0501; using known types for Of or for the key subtype check would hide the discrepancy. J1/J2 preserve all three distinct operations. |
| Soundness: forgotten empty field | T-I, normalize Gone(A) before substituting Never | Accepted E4 is not an equality for an open removed field. J8 retains Without until a fixed inhabitation check allows erasure. This is a correction to T3, not merely a missing visitor. |
| Soundness: arbitrary string passed as a key | T-D with a copied key; two same-K slots called with `"a"` and `"b"`; self-call exchanging K and J | Retain name identity, repeated-key equality and recursive position checks. Solver String checks alone are insufficient. No rule in T2 overrides these bans. |
| Cannot be built with the proposed tables | `pick(K) = (r: { K: Int, "K": String }, key: K) { record.get(r, key) }` | Plain shown-label Strings cannot preserve both labels, and body String slots cannot recover which call slots use K. Add structured shown labels plus scheme key-slot/parameter metadata; no new solver key type. |
| Cannot be built by printing HIR | T-A exported as an alias, private r renamed, then port changed | `package.rs:1316–1323` prints the written alias body. An Of source spelling gives the wrong identity. Use T2's completed alias table with T1/T3's full writer. |
| Source conflict: foreign Codec | T-H's consumer re-extracts only decode from an open source | A skipped encode part does not waive R7. Keep the source-module alias and reject the foreign extraction. Remove the unconditional T3 stability promise. |
| Source conflict: syntax | `x()`, `{ x }`, `{ x: "a" }`, `@m()`, `@main()` | Macro-call, group, record, reserved macro attribute, and invalid built-in call respectively. The older macro draft cannot change these accepted readings. |

The quoted source sentences and code paths behind these findings are in §§2–6 and the correction list. Repairs were made in this note only.

### Pass 2: attack the repairs

The bound repair initially said how to constrain the target but did not state where the inferred field result came from. Add p/q to T-B: after reading the fitted value through a tuple/constructor, `q: p = "b"` must still accept. The code already obtains inferred field parts from bound forms and known parts separately (`infer.rs:1636–1678`). J2 and replacement rule 13 now say both; a later T2 read cannot accidentally inherit the narrow known part.

The first J8 wording said “structurally inhabited” without fixing its treatment of label collisions. Individually inhabited fields do not suffice: `{ K: Int, J: String }` can be empty when K and J become the same label. J8 now uses a fixed conservative predicate and refuses parameter-labelled records and unknown intersections. It never proves inhabitation from current weak bounds. This prevents the repair itself from recreating the substitution bug.

The first canonicalization wording renamed labels before sorting, but did not order ordinary-variable naming relative to that sort. Renaming K/J to Z/A in records whose fields contain distinct ordinary variables would still change their numbering. §4 now specifies label renaming, field sorting, then ordinary-variable naming. A small executable model checked alpha-renaming and source-field reordering. The review also separated the invalid local-alias fixture from runnable T-C and made T-D's call a value binding, since a lowercase call at item level is reserved macro syntax.

### Pass 3: clean semantic pass

Attack all repaired rules together, not only the examples that first failed:

- Substitute Never, a singleton, and a whole scalar into T-I, in both function polarities and inside an invariant wrapper. Retained Without preserves the open dependency; E1 handles the empty case; permitted E4 handles the singleton/whole case. Check E3 nested forgetting and a forgotten-label set containing both a fixed label and a parameter: the original label-equality guard still applies. No new equation needs a nonempty bound.
- Put a scalar complement, a record, and a completed record alias in one multi-clause match. Scalar Except keeps its head guard; all record tests take the same domain; rule 12 removes all records after the covering record clause. A field mismatch remains an error even when coverage itself succeeds. Contrast a one-clause choice, which is fitted and may need Assert.
- Repeat T-D with copied keys, repeated keys, exchanged recursive keys, inner callbacks, Of(key), and Of(label-generic callee). None gains a route around L2/L4/L5/L6. Check a valid effectful singleton key with reordered call labels: it is inferred once and evaluated once in written order.
- Run the T2 argument on paper with `Codec(A, A)` instead of `Codec(D, E)`: demanding one side fixes the shared variable; skipping the other side cannot detach it. An unfilled source and a generalized source still fail differently. A later incompatible constraint remains active through reverse links. Record fields add no privilege to the snapshot.
- Keep K1's invariant variable inside a Rec field and inside a Without base. The variance walk still reaches both occurrences; J8 never erases an unresolved discarded field to hide it. A type read cannot use a printed Never to declare that variable closed.

No further serious semantic issue found in this pass. This is a design argument, not a proof of the future solver implementation or of all principal-type properties.

### Pass 4: clean code-feasibility and source pass

Trace T-A from HIR through type dependencies, snapshot/closure, alias_types, type tests, query text and package text; trace T-D from resolved key identities through both scheme instances, expected/known matching, argument permutation and lowering. Every boundary now has an owned representation, including the metadata that cannot be recovered from Ty. T3's closure/known visitors and transaction state are explicit gates, not default match arms.

Recheck group ordering against `infer.rs:233–355`, bound mutation against `solve.rs:265–285,385–447,577–633`, and K1 against its changed invariant walk. No required operation depends on the current package writer parsing hover text, on inventing runtime Of values, or on querying a partly completed imported source. The record runtime work remains exactly T3's assigned implementation, including its empty access-list fix; no new runtime feature is needed for T2.

Recheck the full grammar against syntax §3, T1 §6, T2 P1–P5, T3 §§8.1–8.3 and A1's reserved forms. Check all A1/T1/T2/T3 code ranges, all spec-section ownership rows, and all shared-file merge orders against the final ledger read. Built-in attributes and qualified constructor bindings keep their accepted meanings. The only remaining coordinator choice is the explicit E4 correction in Q1. No further serious implementation or source conflict found in this pass.

### What was actually checked

- Read the controlling sources, the named code regions, macro draft and K1 diff/fixtures. The baseline code cannot establish the future new-syntax outcomes.
- Ran an in-memory Python set model over all nine records with optional a/b fields taking 0 or 1, and all 16 choices of the two field-value subsets. It reproduced open E4's empty-field counterexample; every closed case obeyed the guarded rewrite. This finite model is evidence for the counterexample and guard, not a full language model.
- Ran a small canonicalization model with K/J versus Z/A, reversed field order, and renamed ordinary field variables. The corrected ordering produced identical canonical forms; fixed and parameter labels remained distinct.
- Checked 20 quoted old-text fragments against the three actual note files; all matched after whitespace normalization. The two declaration/code fragments were read directly. Checked Markdown fences, named files/anchors, reserved error ranges, whitespace, and the final changed-file list.
- No Rust implementation, new-language compilation, Wasm execution, full verification suite, performance run, or external review was performed. K1's landed verification is the coordinator's report in the ledger, not a run by this lane. Every expected compiler diagnostic/runtime value in §8 is a gate for the implementing track.

## 12. Risks and coordinator question

**Q1. Accept the E4 correction in J8 before T3 implements record normalization? Recommended answer: yes.** The accepted set definition, empty-type law and principal-type requirement take precedence over the open-field rewrite's informal justification. Use the fixed conservative inhabitation predicate, retain Without otherwise, and make T-I plus the actual Schema/Codec builders a pre-merge gate. Do not add nonempty constraints or make normalization depend on polarity. If the retained form causes an important builder to fail, report that concrete program; do not restore the inexact rewrite. This answer changes `records.md` §§4.3/12 and the corresponding T2 M1 reference listed in §10.

There is no remaining language-choice question here about Of versus known facts, foreign partial reads, macro call/group syntax, key bans, one-clause choices, or the reserved error blocks: the controlling sources already decide them. The metadata and shared-file corrections are implementation requirements for this joint design. Macro Types.members retains T1's API decomposition, distinct from solver membership; M4 owns its implementation and limits.

The principal remaining risks are implementation size and conservative rejection. Retained Without can make some previously promised simplifications unavailable, including a closed nominal discarded field that the small inhabitation predicate cannot prove nonempty. The forms remain exact, but T3's incomplete subtyping may reject more correct programs; its tests must measure that effect. T2's transaction must cover T1's fused bounds and T3's label/record propagation without weakening ownership checks. Full type trees and canonical ordering must be shared by tooling; limited hover output cannot serve as a cache or interface. Existing T1 known-type limitations for callbacks that change the entire constructor family stay in its RF scope; extending record traversal does not solve that separate problem.

Acceptance of this note establishes the shared contracts and the correction list. It does not claim that all three implementations exist or that their combined principal inference, runtime correctness, scale, or diagnostics have been verified. The ordered gates in §9 remain necessary.
