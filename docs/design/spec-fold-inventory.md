# Spec fold-in inventory

Inventory of the decisions in `docs/design/*.md` against `docs/spec.md`
(2026-11-13, branch lane/spec1; the spec as merged at the integration head f5680600, then folded on this branch).

Columns:

- **Status**: `USER` = an entry marked USER in the ledger; `RELAYED` = an
  entry marked RELAYED in the ledger; `owner` = recorded as an owner decision
  in the design note itself (no ledger entry of its own); `coord` = settled by
  the coordinator or a draft author under the owner's delegation ("不用什么都问我",
  O22) or a review ruling; `proposal` = never accepted; `superseded by X`.
- **Spec**: `yes §n` = `docs/spec.md` states it; `staged` = written in this
  lane as a staged `i future-SYN3` section; `no`; `n/a` = not spec material
  (implementation, tooling, measurement, library-only API).
- **New syntax**: `yes` = the normative text or its examples need syntax that
  the current parser rejects (syntax three); `no` = expressible today.

Ledger entries are cited by their time (`L 11-12 23:08` = the entry of
2026-11-12 23:08).

## syntax-3.md (syntax three, services and effects, generics)

All rows below are staged in `docs/spec.md` by this lane, except where noted.

| Decision | Status | Spec | New syntax |
|---|---|---|---|
| O1 one meaning per symbol: `{ }` computation, `( )` data, `->` result, `\|` join, `=` bind, `:` type | owner, RELAYED L 11-12 18:48 (USER) | staged | yes |
| O2 parameters always bracketed; the parameter group is a data pattern | owner (L 18:48) | staged | yes |
| O3 type first, each branch may carry its type | owner | staged | yes |
| O4 functions joined by `\|` are one function; first written wins; clause blocks removed | owner | staged | yes |
| O5 overloading = chains; by arity, exact | owner | staged | yes |
| O6 precision at the call site; fallback to the union; no first-class intersections of function types | owner | staged (selection §4.8 rule 5a; full selection table of note §2.6 summarised) | yes |
| O7 data = finite map of keys; exact shapes; open records deleted | owner | staged (open records deletion is stage 6) | yes |
| O8 call brackets make data; `(x)` is not `x`; `x \|> f` is `f(x)` | owner | staged | yes |
| O9 `( )` creates no scope | RELAYED L 19:19 | staged | yes |
| O10 labels in data types `label = T`; `:` only name-then-type | RELAYED L 19:22 | staged | yes |
| O11 `(name : T)` in a parameter is a positional binder; labels only by `key = pattern` | owner (answers D1) | staged | yes |
| O12 no punning | owner (answers D2) | staged | yes |
| O13 destructuring left to right; `=` chains; structure before name | owner (answers B4) | staged | yes |
| O14 one form of type definition `Name(params) = right side`; labelled types; one constructor each; sums are unions; `T { }` removed | owner | staged | yes |
| O15 `\|>` weakest operator | owner | staged; also lane/s5 moves it on today's syntax (not landed) | no (level change only) |
| O16 environment reads `db ! Db` | superseded by O24/O29 | no (removed) | — |
| O17 guard inside the parameter group | RELAYED L 19:03 | staged | yes |
| O18 multi-shape function is a first-class value; run time supports it | RELAYED L 19:06 | staged | yes |
| O19 costs accepted: no row-polymorphic field functions, no field addition | owner | staged (by omission) | yes |
| O20 Lisp body any expression; labels in fixed order; printed value is a legal expression; `{ (Float) \| (Float, Float) } -> Float` | RELAYED L 20:45 | staged (text form, multi-shape type); Lisp rows: not staged (spec has no Lisp section, see meta.md) | yes |
| O21 sum members need not be constructors; `i/syntax` drops `Int`/`Float` wrappers; `Node = (Name \| Int \| …)` | RELAYED L 20:51, L 20:53 | staged (general rule); `i/syntax` itself is not in the spec | yes |
| Built-in types are exports of `i/core` (`Int = core.Int`) | RELAYED L 21:02 (overrides the note's §1.3a "no qualified names") | staged (follows the ledger, not the note) | yes |
| O23 type parameters on function values and types `<A, N>(…)`; binding heads `name(A) =` removed | USER L 23:08 (effects proposal) | staged | yes |
| O24 `!` → `<-`; `!` and `^` leave the language | USER L 23:08 | staged | yes |
| O25 a call using only the current environment is unmarked | USER L 23:08 | staged | yes |
| O26 no implicit capabilities | USER L 23:08 | staged | yes |
| O27 `^ E` abolished; errors are the service `Fail(E)`; services keyed by type constructor, no order, names, repeats | USER L 23:08 | staged | yes |
| O28 Effect-TS model, rules R1–R6; `..env` replaces `keep_env`; layers are ordinary functions | USER L 23:08 | staged | yes |
| O29 provide `e <- (s)` for any `e`; block form `{ … } <- (s)` | RELAYED L 23:29, L 23:30 | staged | yes |
| `<-` provide is the weakest binary level, left associative, right operand one postfix expression | coord L 23:30 | staged | yes |
| O30 `catch` uses a light escape-only boundary; bench gate ≤1.25x | RELAYED L 23:29 | staged (semantics only; the gate is not spec material) | yes |
| O31 platform provides a top-level `Fail` at entries | RELAYED L 23:29 | staged | yes |
| O32 per-target defaults chosen explicitly in `main`; no package supplies services implicitly; provider plans and `i.toml` `providers` deleted | USER L 23:08, RELAYED L 23:29 | staged; old §2.6.1 deletion is another lane's | yes |
| §1.3 author rules (Opaque position and invariance, `A(..)` whole-sum export, rest needs known shape, D3 `\|` joins only function literals, D5 `None` is a value / `None()` error, D7 display by declared name, D9 empty match `core.never`, D10 stored overload result union, D11 single-shape host exports/entries, solver checks selected domains, first branch wins at run time, `Nil = ()`, no default parameters) | coord ("accepted without explicit owner confirmation") | staged | yes |
| §1.3a fourth round Q1–Q19 (no positional after labelled, UTF-8 byte label order, no rest at top of a group, …) | coord (L 20:53, RELAYED "不用什么都问我") | staged | yes |
| Q13 "no qualified names for built-in types" | superseded by L 21:02 (`Int = core.Int`) | — | — |
| Fail in the prelude; `raise`/`catch` in `lib/i/fail.i`; light boundary detected from the literal passed to `control.boundary`; `task.Scope(E)` renamed `task.Group(E)`; one entry per target when targets need different services | coord (L 00:14, note §2.10) | staged (all but the boundary-detection mechanism, which is implementation) | yes |
| §8 open item 1: `Node(..)` does not export the members of its data's union | coord (recommended, open) | staged as recommended | yes |
| Stage plan, migrator, sizes, measurements (§6, §7) | coord | n/a | — |

## syntax-2.md (SYN2)

| Decision | Status | Spec | New syntax |
|---|---|---|---|
| Change 1 postfix match `e.{ }` | superseded by USER L 11-11 "e.{} 废除" | no (correct) | — |
| Change 2 bare pipe `\|` replacing `\|>` | superseded by USER L 11-11 (pipe stays `\|>`) | no (correct) | — |
| Change 3 error type `^`, `?` only a guard | USER L 11-11; `^` then superseded by O24/O27 (syntax three) | yes §3, §4.6 (today); staged replacement removes it | no today |
| Change 4 records and record types in parentheses `(name : v)` | USER L 11-11; superseded by O7/O10 (data with `label = v`, no open records) | yes (today); staged replacement | no today |
| Change 5 tuples positional, at least one item, no label mixing | USER L 11-11; superseded by O8 (data mixes positions and labels) | yes (today); staged replacement | no today |
| Change 6 empty record `()`, type `Record`, `()` forbidden in types | USER L 11-11; superseded by O7/§5.2.2 (`()` is the empty data type, `Record` removed) | yes (today); staged replacement | no today |
| Change 7 `@main` removed; `main` of `src/main.i`; `[entries]` | USER L 11-11, refined by USER L 11-12 (no file runs, no `--entry`, `i run NAME`) | yes §2.6, §10.2 | no |
| Change 8 tests by `test("name", () { … })` macro; `@test` removed | USER L 11-11 | yes §10.2 (tests bullet) | no |
| Never resolve an ambiguity by adding a comma | USER L 11-11 | partly (spec §1.7 has the line rules; no explicit sentence) | no |
| `{ T }` type groups kept; mixed call arguments kept (§10 q3, q5) | coord | yes §3, §6.6 (today); call-argument rule superseded by O8 | no |

## syntax.md (syntax without keywords, A1–A3)

| Decision | Status | Spec | New syntax |
|---|---|---|---|
| No keywords | owner (decisions taken with the user) | yes §1.4 | no |
| Imports and exports are groups told apart by content | owner | yes §2.2–§2.4 | no |
| Clause blocks `{ p -> e }`, `e \|> { … }` matching | owner; superseded by O4 (chains) | yes (today); staged replacement | — |
| Guard with `?` | owner; position changed by O17 | yes (today); staged replacement | — |
| One-clause unguarded clause block is a binding (reports to `Assert`) | owner; carried into the staged "group with one branch" rule | yes §6.12 (today); staged | yes |
| Type variables declared after the binding name `map(A, B) =` | owner; superseded by O23 | yes §2.5, §4.7 (today); staged replacement | — |
| Records `(name : e)`, `Record` | superseded (see syntax-2, syntax-3) | — | — |
| Lowercase types read the type of a value; alias patterns (T2) | coord (T2) | yes §3, §4.3 | no |
| Forms reserved for macros are compile errors | superseded by meta.md (macros landed, M1–M4) | yes §1.4, §2.2, §3 — **stale** | no |
| A1/A2/A3 migration notes | historical | n/a | — |

## meta.md (macros and the Lisp surface)

| Decision | Status | Spec | New syntax |
|---|---|---|---|
| One core tree, two surfaces; `.i` normal, `.is` Lisp; one surface per file; both files for one module is an error | owner ("decisions taken with the user") | yes §1.1, §2.1, §14.1 (folded on this branch) | no |
| Macros defined only in `.is`, used from both surfaces; `@macro` in `.i` is an error | owner | yes §14.5 (folded on this branch; §1.4, §2.2, §3 updated) | no |
| Lisp surface: no keywords; symbol of a form is its head; quote `` ` ``, unquote `~`, `~..` | owner | yes §14.2–§14.4 | no |
| Decisions 1–15 (function `(-> (params) R body)`, imports/exports as `(, …)`, `@m(args) Item`, macro arguments in types, no statement or pattern results, macro not usable in its own module, `Syntax` with no constructors, `make` hygiene, `Definitions`/`Types` services, `i/syntax` unreachable from entries, typed macros as declaration + binding, result-only type variables met by `Never`, no delayed erroneous argument) | owner | yes §14 | no |
| Decision 6 `a \| m` is the macro call `m(a)` | superseded by USER L 11-11 pipe withdrawal (`a \|> m`) | no | — |
| Lisp rows for syntax three: parameters as data `(-> (, a b) R e)`, `(,)`, pipe head `\|>`, chain `(\| …)` | coord (syntax-3 §6.8, fourth round) | staged (§14) | yes |
| Lisp-convention layout of `.is` (closers gathered on the last line, Lisp indentation, spaces) | USER L 11-12 23:41 | yes §1.7 (`i fmt` paragraph, from lane/syn2) | no |
| Macro budgets (macro-budget.md: 100,000,000 work units, 2^28 words, 10,000 frames, nesting 256, per call) | coord | yes §14.5 | no |
| Hygiene marks, definitions queries, typed-macro boundary (§5–§7), M4 typed macros | coord / implementation | yes §14.6–§14.9 | no |

The macro and Lisp-surface design is now §14 of the spec (today's rows), with
a staged syntax-three version of §14 beside it.

## records.md (T3 structural records)

| Decision | Status | Spec | New syntax |
|---|---|---|---|
| D1–D10 open records, width subtyping, `Without`, label parameters, `i/record`, run-time tag 14 | coord (G3, landed T3); **superseded** by O7/O19 (open records deleted, syntax three stage 6) | yes §3, §4, §6.13, §14 (today); staged text removes them | — |
| §20 substitution stability, §21 Elixir BDD algorithms | coord; superseded with the above | n/a | — |

## value-types.md (T2) and literal-types.md (T1) and types-joint.md

| Decision | Status | Spec | New syntax |
|---|---|---|---|
| Value types (lowercase names in types), alias patterns `Schema(User, _) = user`, E0530–E0537, explicit fixation | coord (G2/G3, landed T2) | yes §3, §4.3, §4.7 | no; staged §4.3 keeps alias patterns |
| Literal types for Int and String; no widening; `-` on literals; W0503 advisory disjoint equality | coord (landed T1, W520 amendment) | yes §4.2 "Literal types", §4.8 | no |
| Joint representation, rule numbering, error allocation | coord | yes (rules as numbered in §4.5/§4.8) | no |

## js.md (JS bindings, target providers, checked WASI)

| Decision | Status | Spec | New syntax |
|---|---|---|---|
| J1: a target with no default implementation of a needed service is a compile error even if the error is caught | USER L 11-10 | yes §2.6.1 (today); staged §10.2 keeps it as "a need of an entry the target does not provide is a compile error" | no |
| §2 target provider plans (roles, adapters, seeds, `i deps --target`), package-owned defaults | coord (J2–J8, K1–K5, L1–L4); **superseded** by O28/O32 (USER L 11-12 23:08: provider plans and `providers` deleted) | yes §2.6.1 (today, to be deleted by another lane) | — |
| §3 foreign declarations `@js`, linking, loader lifecycle | coord | yes §10.1.1, §10.1.2 | no |
| §4 synchronous bridge and conversion | coord | yes §10.3, App. B.6.1 | no |
| §5 requests, cancellation, IO ownership on JS | coord | yes §9.11 | no |
| §6 checked WASI adapter | coord (WASI exception accepted) | yes App. B.7.1 | no |
| JS step 02: `std/http.Client` provider defaults in `std/i.toml` | coord; superseded by O32 | n/a (library) | — |

## native.md (native packages and multicore workers)

| Decision | Status | Spec | New syntax |
|---|---|---|---|
| N1 native is the default target of `i build` | USER ("Native proposal ADOPTED", §8 item 4) | yes §2.6.2 table | no |
| N2 a native package is a prebuilt runner plus AOT program bytes; one executable | USER (adopted) + lane/pack implementation (L 11-13 02:01) | yes §2.6.2 (folded on this branch) | no |
| Executables written by `i build` are statically linked | USER L 11-13 02:04 | yes §2.6.2 (folded on this branch) | no |
| `--target` values `native` (default), `js`, `nodejs`, `wasi`, `host` (`ilang` alias); plain `i build` no longer writes `.wasm` | USER (N1) + lane/pack | yes §2.6.2 | no |
| N3 native uses ABI v3 wire, not WASI | coord | n/a (host implementation) | — |
| N4 reject only required imports the runner cannot supply | coord | yes §10.1.2 (link rule) | no |
| N5–N8, N10–N12 one Store per worker, tasks never move, one shared queue encoded at `go`, one lock, FIFO fast path, one encoded format, a stop closes the whole run | USER (adopted) / coord | partly: §11.9 states the observable rules (workers cannot be seen, first stop wins); the mechanisms are n/a | no |
| N9 IO values are private nominal wrappers over Int; ResourceContext; numbers never reissued | USER (adopted §8 item 2) | yes §9.12 (wrappers), mechanisms n/a | no |
| "No TLS, UDP, HTTP or DNS in v1" | superseded by USER L 11-11 (DNS, TLS, HTTP client delivered) and L 11-12 17:55 (UDP) | yes §9.7, §9.12 (`i/tls`, UDP in `i/net`) | no |

## multicore.md

| Decision | Status | Spec | New syntax |
|---|---|---|---|
| D1 one semantic rule (steps in some order), D2 portability checked at run time, D3 `core.portable`, D10 order from the task tree, D11 first stop wins, D12 worker count invisible | coord (gate G2) | yes §11.9, §13 | no |
| D4 `control.keep_env` | coord; superseded at stage E6 by `..env` service patterns and library `keep` (O28) | yes §9.4 (today); staged §9.4 states `keep` | yes |
| D5–D7, D9, D13 scheduler and copy mechanics | coord | n/a | — |
| D8 cooperative stealing | superseded by native N7 (shared queue at `go`) | n/a | — |
| §8 `share` | coord | yes §9.9 (task table) | no |

## runtime.md (random, tasks, IO)

| Decision | Status | Spec | New syntax |
|---|---|---|---|
| D1 random is immutable explicit data (SplitMix) | coord | yes §9.10 | no |
| D2 `Request`/`Ticket` in `i/request`; D3 `Await` service; D4 one root hand-off | coord | yes §9.11 | no |
| D5 provision entries by the complete provider plan | coord; superseded by O32 (platform table, explicit `main`) | yes §2.6.1, §10.2 (today); staged §10.2 | yes |
| D6 scopes by module functions; D7 lifetime/portability; D8 cancel by resumption; D10 test scheduler | coord | yes §9.9 | no |
| `task.Scope(E)` | superseded by `task.Group(E)` (syntax-3 §2.10, coord) | staged | yes |
| D9 IO resources private nominal wrappers over Int | coord, confirmed by native N9 | yes §9.12 | no |
| `task.ref`/`task.update` as the only state construct; immutability | USER L 11-11 (two entries) | yes §9.9 | no |

## host-concurrency.md (ABI v3)

| Decision | Status | Spec | New syntax |
|---|---|---|---|
| D1–D3, D12 request ids, status 1/2, `ilang_continue`, `cancel(id)` | coord (landed stream C) | yes App. B.4–B.5 | no |
| D4 no Wasmtime fibers; D5 one `ilang` thread with tokio | coord; "needs the user's approval" in the note, no USER entry found | n/a (host implementation) | — |

## numeric-lowering.md, lazy-reentry.md

| Decision | Status | Spec | New syntax |
|---|---|---|---|
| Numeric lowering without boxes, O2/O3, budgets (§9) | coord | n/a (no observable change) | — |
| Lazy re-entry D1–D18 (D12: no spec change) | coord | n/a | — |

## Library and tooling notes

| Document | Decision | Status | Spec | New syntax |
|---|---|---|---|---|
| testing-macro.md | `std/test` item macros `test`, `suite`, `skip`, `only`, `fails`, `each`; `[test] collection`; fresh instance per test; `@test` removed | USER L 11-11 | yes §10.2 (summary) | no |
| unused.md | unused top-level binding warning; direct references count | USER L 11-11 + coord | yes §2.4.1 | no |
| macro-budget.md | per-call macro budgets | coord | no (with meta.md) | no |
| tls.md | `i/tls` connect/accept/dial, options, `TlsError` | USER L 11-11 (TLS delivered) | partly §9.12 `i/tls` | no |
| http.md | `std/http` client, `Client` service via provider-plan cells | USER L 11-11; provider selection superseded by O32 | n/a (std library) | — |
| std-1/2/3.md | pure std modules, time/log, property tests, strings | coord | n/a (std library, not `lib/i`) | — |
| str-report.md | string interpolation, raw and multi-line strings | coord (landed STR) | yes §1.6 | no |
| a1-front-end.md, reports (`*-report.md`, `dev-verify-report.md`, `test-migrate-report.md`), prior-art-*.md, macro-status.md | historical, studies and status | n/a | — | — |

## For the owner

The coordinator answered the first list on 2026-11-13: built-in types follow the
ledger (`core.Int`); meta.md is folded (§14); `share` was already in §9.9; the
syntax-3 rules of its §1.3 are decided; `T(..)` follows D8 and the note's §8
recommendation; the comma principle and the performance gates stay out of the
spec. One question stays open:

1. host-concurrency.md D4 (no Wasmtime fibers) asked for the user's explicit approval and has no USER entry; it is implemented and is not spec material. Is it approved?
