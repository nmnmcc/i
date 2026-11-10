# T3 stage record

The baseline is `f6f9cc3a2ea06df397e369b720ce5aed81fedbda` on `lane/a`.
The lane was clean on entry: there was no interrupted work to keep or discard.
Cargo metadata reports this lane as its workspace root. All writes stay in
this lane. T2 has not yet landed on the baseline.

## Stage 1: specification

The T3 replacement paragraphs from records §12 are installed in `docs/spec.md`,
with joint §6's shared wording. Sections 4.9, 6.13 and 14 were free. T1's subtype
rule 8 stays; records take rule 9. T2's sections are left for its merge.
The implementation is still the baseline compiler at this checkpoint.

Interpretation sources:

- Joint §6 says: “Apply the individual accepted replacement texts only where
  the table says to”. Only T3's portions are installed now.
- Ledger G3 says: “the conservative E4 correction is approved, with the
  substitution and Schema/Codec tests required before T3 merges”. This resolves
  records §4.3's stale open-field rewrite. Spec 4.9.2 retains forgetting when
  the discarded field is not provably inhabited for all substitutions.
- Joint J2 says: “Rule 13b has only its principal constraint”; constructor and
  tuple access retain T1's separate bound and known-result rules.
- Records D10 says: “Out of T3: record patterns”. Pattern work means head tests,
  narrowing and coverage, without record-shaped patterns.
- Joint §9 says: “shared work rebases onto T2 and merges after T2”. Runtime
  ordering and representation can be developed while that integration finishes.
- Records §12 says IO owns sentences counting library modules. The existing
  opening count is preserved; updating it after adding i/record is an IO
  escalation. Module paths and section pointers are installed by T3.

The new 6.13 example has an explicit T3 future entry in old-syntax-check.json.
Its fence is marked `future-T3` while record syntax is pending. This follows
literal-types §12 step 1: “the blocks that write literal types ... are marked
as not yet checked, and step 8 removes the marks”. The old-syntax checker
still parses the record example and requires its exact reservation errors.
Only this new pending example has the marker; every pre-existing spec fence
keeps its original parser and formatter checks. Remove the marker and future
entry when the record parser lands.

Checks and incidents:

- Cargo metadata: workspace_root is /home/agent/Programming/i/.rd/a.
- First full dev-verify: failed in format_properties because the new pending
  example changed the exact unparsable-source inventory.
- Two full runs with the future fence passed, including all 224 spec cases
  at O0/O2, workspace tests, i tests at O0/O2, the WASI smoke check, JavaScript
  SDK tests and 51 browser tests on port 4181.
- To retain the ordinary fence, the formatter inventory was regenerated with
  `UPDATE=1 cargo test -p ilang-syntax --test format format_properties --locked`.
  That test passed and generated one added line, parse docs/spec.md:1028.
  A full run then passed format_properties but failed spec_blocks_parse:
  the normal fence still requires parser support. No test was changed.
- The future marker was restored using the staged-spec rule above. The
  inventory was regenerated again with the same command to restore its
  baseline contents. No golden change is intended in this checkpoint.
- old-syntax-check: passed, 1,337 snippets, 18 future-track entries. It still
  checks the pending record example's errors, including E0114.
- Final full gate on the restored future fence: passed (exit 0), through
  `/home/agent/Programming/i/.rd/cache/heavy direnv exec
  /home/agent/Programming/i env ILANG_BROWSER_PORT=4181 ./scripts/dev-verify`.
  Both spec-block parsing and formatter properties passed. All 224 existing
  spec cases passed at O0/O2, as did the library/bench/example tests, WASI
  smoke, JavaScript SDK checks, and 51 browser tests.
- Final old-syntax-check: passed (exit 0), 1,337 snippets and 18 T3 future
  entries. The no-T3-future-entry done criterion is not met.
- `git diff --exit-code -- crates/ilang-syntax/tests/format_skip.txt` passed:
  the generated inventory is back at baseline. No golden change remains,
  so this checkpoint needs no derived-files commit.
- `git diff --check` passed. Verification left only ordinary lane build and
  npm dependency artifacts. UPDATE and ILANG_BROWSER_PORT were command-scoped;
  no persistent environment setting changed. No check was interrupted.


## Source conflict before stage 2

No compiler changes have started. The next rule cannot be implemented with
all of the required equalities. This conflict is separate from open-field E4;
the ledger's approved J8 correction does not resolve it.

Minimal closed alias: `Gone = { ..{ a: Never } | { b: Never }, -a }`.
The complete acceptance probe adds `empty = (x: Gone) { x |> {} }`.
No imports, value reads, label parameters, or inference variables are involved.

Controlling sentences:

- Spec §4.1: “Every type is a set of values.” Spec §4.4: “The values of
  `A | B` are the values of `A` and the values of `B`.”
- Records §3: “A record type with a field of type `Never` has no values.
  It is kept as it is and not turned into `Never`”.
- Records §4.5: “`Rec(fs) | Rec(gs)` is `Rec` of the labels that are the
  same in both, with their types merged by `|`.” That paragraph explicitly
  says the merged type is what a written union means; the conflict below
  does not assume that written unions remain unmerged in the solver.
- Records §4.3 E1 gives Never for a Rec with a plainly empty field.
  E2 says `without(A | B, L)` is `without(A, L) | without(B, L)`.
  The same section requires: “Each rewrite is an equality”.

There are two incompatible results:

1. Join first. The record alternatives have no common labels, so the base
   becomes `{..}`. Forgetting `a` from it is `{..}` by E4. J8's proof of
   inhabitation is vacuous: no fields are discarded. Gone contains the empty
   record, and the empty clause block of the acceptance probe must be rejected
   by spec §4.8 rule 11.
2. Use E2 first. Each alternative is plainly empty, so both branches become
   Never by E1. Gone is Never. The acceptance probe has an impossible input,
   and its empty clause block is valid by the same rule 11.

`tests/t3-forgetting-conflict.py` is an interpretation check: it calculates
these two source-mandated paths over a finite universe containing the empty
record, and fails their required equality. It is a substitute mathematical
witness, not an implementation test or a claim that T3 syntax already works.
The values follow the quoted rules; no golden or expected compiler output
was generated. The existing compiler still reserves record syntax. The closed witness failed its
required equality (exit 1); the extended canonical-union witness also failed
joint §6 substitution stability (exit 1). Both are interpretation checks.

A fixed evaluation order does not make E2 an equality and does not satisfy
the set meaning of forgetting. Discarding only closed empty alternatives also
needs an open-type audit: joining `{ a: A } | { b: B }` before substituting
`A = Never, B = Never` loses both variables. J8's exact-substitution gate makes
that loss observable. A coordinator decision must reconcile record joins,
empty/possibly empty record members, and E1/E2 before implementation resumes.

The same conflict occurs when E2 sees a canonical union, so restricting E2
to solver Union nodes does not resolve it. Use this generic alias:

`Gone(A) = { ..{ a: Never } | { ..A, b: Never }, -a }`.

- Before substitution, the base has one Rec member and one spread member,
  which §4.5 requires keeping separately. Outer E2 removes the Rec branch
  by E1 and leaves `Without({ ..A, b: Never }, {a})`.
- Substitute `A = {..}` in this normalized form. The inner spread becomes
  `{ b: Never }`; outer E1 then gives Never.
- Substitute `A = {..}` first instead. The outer base is now
  `{ a: Never } | { b: Never }`. The required join produces `{..}`; outer
  forgetting gives `{..}`.

Every E4 step here erases no fields, so the approved J8 guard permits them.
No unresolved discarded field or label parameter is involved. This contradicts
joint §6's requirement: “Normalization before or after substituting a closed
type must describe the same set.” The record/spread union in the generic alias
is in an output place, so the spread-place restriction does not reject it.

Outstanding gates: implementation and all T3 compiler/runtime/spec probes,
principal-type and order probes, scale timings, WAT comparison, std +10% gate,
no-T3-future-entry gate, and the Schema/Codec integration example. No timing or
WAT result is claimed for T3. Redesign remained f6f9cc3 at the last check.


## Resumption at 0d0ce19: accepted exact-record contract

The lane was clean on resumption. Its compiler includes T1 and T2; redesign
is 1a8fee2 and is already an ancestor. The old conflict above is historical:
records §§20–21 replace the common-field union with exact alternatives and
N2. The original failing model is retained as a negative control, not a
requirement for the new compiler to reproduce its old failure.

Read before this stage: revised records.md, including the change/rejection
lists and reference-first hand-off; the type study at 3beae77 and compiler
study at 3cc1def; the reference INDEX's type/records, coverage, diagnostics
and interface routes; shared.md's “Prior art first” instructions. The two
study files are absent from this checkout. Both were read with git show at
the accepted revisions, rather than silently substituting another file.
Records §21.1 documents the same checkout provenance. No reference tree was
edited, built or copied verbatim.

Models rerun successfully through the dev shell:

- t3-record-substitution.py: 4,994,560 substitutions, zero new-rule failures;
  33,344 independent set checks; 78,040 union-order and 1,024 label checks.
  The old rules still show 32,235 semantic commutation failures.
- t3-record-demand.py: 874,752 compact commutations; 133,120 inclusion checks;
  15,876 extended substitutions and 4,202 independent set checks. Both
  20-step model families passed; these are model timings, not compiler timings.
- t3-record-bdd.py: 38,400 Boolean pairs with caches on/off, 133,120 map
  inclusions and 16,384 negative projections, zero oracle disagreements;
  reordered atoms, substitution, sharing and resource retry checks passed.

The coordinator allocated E0546 to resource exhaustion and assigned the
versioned graph codec to T3. Arbitrary heterogeneous traversal stays deferred;
fixed-schema mapping is required. New tests will not require E0520 to be an
error; W520 owns that policy change.

## Stage 2: label order and the bounded Boolean kernel

Reference code read before implementing these mechanisms:

| File / function | Taken | Required difference in i |
|---|---|---|
| ilang-wasm runtime/values/strings.rs, string_compare and string_from_bytes | Byte comparison followed by length | Verify against Rust str order for all pairs of labels; record source order remains evaluation order |
| ref/elixir/lib/elixir/lib/module/types/descr.ex, bdd_node_new, bdd_union, bdd_intersection, bdd_difference, bdd_negation | Ternary decision meaning (p & C) union U union (not p & D), shallow hash-consing and completed apply results | Safe integer handles; iterative evaluation; no closed-map, gradual-type, recursive-type or library rules |
| Same file, subtype_static?, map_bdd_empty?, init_map_line_empty?, map_line_meet_empty?, map_split_key_line, map_remove_negative | Uncovered-set emptiness, required-empty-field pruning, first-difference products and shared-witness projection | Map/field theory is a later integration stage; do not claim Boolean truth assignments establish record inclusion |
| ref/tsrs/crates/tsrs_checker/src/unioncache.rs, UnionCacheMode and cache contract | Cache-off/shadow validation; completed pure results only | No mutable-bound result cache; one demand budget charges distinct logical states independently of memo hits |
| ref/ghc/compiler/GHC/HsToCore/Pmc/Check.hs, throttle | Resource status separate from logical result | Never return GHC's approximation as a subtype result; use ResourceLimit and eventually E0546 |
| ref/typescript-go/internal/checker/checker.go, removeSubtypes | Early identities, cached reductions and explicit complexity refusal | Use i's fixed 65,536-state budget, not TypeScript's thresholds or a false inclusion verdict |
| ref/elixir/lib/elixir/lib/module/types/descr.ex, bdd_map | Visit Boolean children while changing atoms | Memoize by handle and replace atoms through Boolean apply to restore order, rather than reusing the old decision order after substitution, as §21.2 requires |

The label-order test executes the production runtime in Wasmtime. Its expected
order comes independently from Rust str::cmp, as records §4.1/§16 step 2
requires. The Boolean kernel will first compile only for unit tests, as T1's
pure-algebra stage did. Production type behavior remains unchanged until the
exact graph and solver consumers are connected. Differential tests use the
accepted Python BDD model as an oracle; frontend/compiler differential tests
remain required after syntax and inference are connected.

The kernel uses shallow intern keys and explicit work stacks. The accepted
Python BDD model supplies 19,200 Boolean operation verdicts in each cache
mode; Rust checks each on all eight independent assignments. Substitution
tests change atom order and replace atoms by empty or compound predicates.
A 20,000-decision chain checks native-stack independence. Budget exhaustion
and a successful fresh retry are separate from false Boolean results.
These are kernel checks, not claims about the unfinished frontend, graph
interfaces, field theory, or the real-checker branch-growth/Fan gates.

Stage checks, before merging W520:

- `heavy direnv exec /home/agent/Programming/i cargo test -p ilang-check
  --lib records::bdd --locked`: exit 0, all five tests passed in 0.38s.
  The earlier four-test run also passed, before substitution was added.
- `heavy direnv exec /home/agent/Programming/i cargo test -p ilang-wasm
  --test record_labels --locked`: exit 0, all 225 pair comparisons passed
  in the production runtime; Wasm validation and execution passed.
- `heavy direnv exec /home/agent/Programming/i env
  ILANG_BROWSER_PORT=4181 ./scripts/dev-verify`: exit 0. Formatting, Clippy
  with warnings denied, workspace tests, all 239 existing spec cases at
  O0/O2, 1,465 std tests, two example tests and 12 bench tests at both levels,
  WASI smoke, SDK tests and 51 browser tests passed.
- `direnv exec /home/agent/Programming/i ./scripts/old-syntax-check`:
  exit 0, 1,621 snippets, 25 T3 future entries. A preflight with the existing
  parser binary (a substitute inventory, not the rebuilt-parser gate) found
  seven missing registrations for the newly accepted record examples. They
  now have exact future-track errors, as old-syntax-check.md requires:
  “Their owning track must remove the entry when its syntax lands.” No
  record syntax has landed in this stage. A Rust assertion's message was
  reworded to avoid falsely looking like an i binding to the extractor.
- `direnv exec /home/agent/Programming/i python3 -I
  scripts/old-syntax-check-test`: exit 0, all eight extractor regressions.
- `heavy direnv exec /home/agent/Programming/i env
  WAT_DUMP=/home/agent/Programming/i/.rd/a/target/t3b-wat-baseline
  cargo test -p ilang-cli --test wat_dump --locked`: exit 0, 898 files.
  This is the pre-W520 baseline export, not a before/after byte comparison.
- `git diff --check`: passed. No golden or generated file changed. No cargo
  or gate check failed or was interrupted in this stage. Shared heavy slots
  delayed checks; no slot settings or other worker's processes were changed.
  Environment variables were command-scoped; no persistent setting changed.

Redesign moved from 1a8fee2 to 82375c0 (W520) during verification. The stage
checkpoint precedes that merge; the merged tree must pass the full gate
again. No new test requires disjoint comparisons to be errors.

## Head-test fixture conflict found during stage 2

The revised change table (§20.5, §§6–11 and §§13–16) says: “The remaining
pass/reject tests in §15.1 therefore keep their stated behavior, subject to
existing joint corrections.” Joint J5 keeps head-only tests. This leaves a
separate contradictory required pass fixture in §15.1:

`f(K) = (r: {..}, key: K) -> Int { r |> { y: { ..{ a: Int }, K: Int } -> 0  _ -> 1 } }`

Records §15.1 puts this under “Correct programs (`// expect: pass`)”. But
records §7 says “every record that can come to that place must be in
`{ name: String }`” for a field-requiring record head test. Spec §7.1 says:
“A type test whose type is a record or spread tests only the record head:
every possible record there must satisfy the written field requirements”.
Spec §4.8 rule 11 requires `B <: T | r - T`; subtraction removes the record
head, not a field-sensitive complement (records §20.5).

Instantiate K with `"a"`. T is `{ a: Int }`. The empty record is in the
annotated input `{..}` but not in T, and the fallback cannot recover it
because the first clause takes every record. Thus the definition must be
rejected by the normative rule and accepted by the required fixture. The
fact that y is unused does not exempt the test from rule 11. For the listed
removal variant, K = `"b"` leaves the same required a field and witness.

`direnv exec /home/agent/Programming/i python3
tests/t3-head-test-conflict.py` checks both refutations using the accepted
BDD map model and proves both field-constrained repairs. It exits 0. This
is a substitute interpretation check, not a compiler acceptance test and
not an assignment of a diagnostic number. No production record compiler
behavior has been chosen against the contradictory pass requirement.

## Stage 2: merged verification and stop

Checkpoint e2d5129 was merged with redesign 82375c0 in b6d2926. The merge
was mechanical; no conflicts were resolved by changing either track's rules.
W520's imported golden changes keep their upstream generation commit and
per-line rule audit in tests/w520.md. T3 produced no golden changes.

On the merged tree:

- `heavy direnv exec /home/agent/Programming/i env
  ILANG_BROWSER_PORT=4181 ./scripts/dev-verify`: exit 0. Formatting, Clippy
  with warnings denied, workspace tests, all 241 existing spec cases at
  O0/O2, the five Boolean kernel tests, the label-order runtime test,
  1,465 std tests, two example tests and 12 bench tests at both levels,
  WASI smoke, SDK tests and all 51 browser tests passed.
- `direnv exec /home/agent/Programming/i ./scripts/old-syntax-check`:
  exit 0, 1,630 snippets parsed with exact expected errors, 25 T3 future
  entries. The no-T3-future-entry criterion is not met.
- `direnv exec /home/agent/Programming/i python3
  tests/t3-head-test-conflict.py`: exit 0. Both broad-input interpretations
  are Refuted and both field-constrained repairs are Proven. The conflict
  survives the merge; W520 does not change head-test typing.
- `git diff --check`: passed. No failed or interrupted cargo/gate checks;
  no persistent environment change; no interrupted worker work on entry.

Stop: the contradictory required pass fixture above still needs a source
decision. No frontend syntax, production record graph/solver, field theory,
query/formatter/interface integration, runtime records or i/record library
has been installed by stage 2. The complete frontend/model differential
tests, principal-type and declaration-order probes, soundness runs, all
deliberate-rejection diagnostics, wrap_union/pair/schema/codec runs, real
checker branch-growth/Fan timings, 1/10/100/1,000-field measurements, std
+10% comparison and final merged-baseline WAT comparison remain required.
The 898-file export above is only a pre-W520 baseline capture. Model/kernel
timings are not substitutes for the real-checker or runtime scale gates.

## Examples corrected

The coordinator resolved the head-test example: “the rule stands and the
example is wrong.” Spec §7.1 and records §7 still require every possible
record to satisfy the tested fields. Section 15.1 now requires the addition
input `{ a: Int, K: Int }` and the removal input `{ a: Int }`. The original
`r: {..}` forms are required rejections. Their exact frontend diagnostics
will be recorded once record syntax and typing are connected; the finite
model is not a substitute for those checks.

Standing coordinator rule: when the spec and a numbered rule agree, correct
an example or expected-output disagreement and continue. Only conflicting
rules or an unbuildable rule require a stop. The earlier stop is historical.

Resumption was clean at ba76bf8; no interrupted work needed preservation.
`git merge refs/heads/redesign` initially reported Already up to date at
82375c0. The type/compiler studies are still absent from this actual ref;
their accepted git objects remain available and were read earlier. Continue
using them and recheck the moving ref rather than block on that provenance.

## Stage 3: exact source graph and record products

References read before implementation:

- Elixir descr.ex, map_literal_intersection and init_map_line_empty?: merge
  required fields and stop at an empty required field. i uses only open
  required source fields; absence predicates remain internal to proofs.
- Elixir descr.ex, bdd_node_new and bdd_compute_hash (stage 2): shallow keys
  and shared children. i adds suspended source Union/Inter/Without nodes
  per §21.2; construction never calls Boolean apply or enumerates DNF.
- MLstruct ConstraintSolver.scala, freshenAbove: one substitution map for
  variables and their bounds, shared across the whole type. i's source graph
  visitor is iterative and memoized by handle; labels use one separate map,
  and replacement reruns N1/N2 and guarded E4 without losing invariant uses.
- tsrs docs/CHECKER.md, the type arena and stable TypeId sections: use safe
  integer handles with shallow payload keys. i does not take static pointers
  or mutable type payloads; graph nodes are immutable and topological.

Guarded E4 follows records §4.3/J8: only syntactically universally inhabited
discarded fields may disappear, independent of live solver bounds. N2 and
exact union retention follow §§20.2/21.2, not MLstruct's common-field join.

Stage 3 results:

- The first core build failed: ValueSet's operation is named meet, not
  inter. Corrected the call; the next ten-test core run passed. The scheme
  sharing probe selects fields by label, rather than assuming intern-ID
  order gives source union order.
- Added tests/t3-record-graph-oracle.py. The accepted eager model's independent
  64-row set interpreter and the accepted compact model agree on 36,328
  source construction/substitution cases. The Rust graph matches both.
  This is a graph-engine differential test, not yet a frontend test.
- `heavy direnv exec /home/agent/Programming/i env
  ILANG_BROWSER_PORT=4181 ./scripts/dev-verify`: exit 0. All 11 record-core
  tests passed, along with formatting, Clippy -D warnings, workspace tests,
  241 existing spec cases O0/O2, the i O0/O2 suites, WASI smoke, SDK tests
  and 51 browser tests. No production record path is connected yet.
- `direnv exec /home/agent/Programming/i ./scripts/old-syntax-check`:
  exit 0, 1,630 snippets, 25 T3 future entries. No new future entry needed.
- The corrected head-test interpretation probe exits 0: broad inputs are
  refuted and field-constrained repairs proven. Exact frontend diagnostics
  for the two rejection fixtures remain pending the frontend connection.
- `git diff --check` passes. No generated/golden change; no interrupted
  check; command-scoped environment only. The separately queued final core
  rerun is redundant with the full gate's 11 passing tests.

The stage 4 consumer audit starts from 296 matches of the prescribed
Ty::Con/Ty::Fn/head/Head search, saved in target/t3-ty-audit-before.txt.
The audit must cover graph children and labels as well as record heads.
Redesign is still 82375c0 and is an ancestor. This checkpoint continues
directly into production graph/solver integration under the standing rule.
