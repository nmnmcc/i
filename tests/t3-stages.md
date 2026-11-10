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
