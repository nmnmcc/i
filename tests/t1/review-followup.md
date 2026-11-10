# T1 independent review follow-up

The independent review at `.rd/cache/gpt/report-review-t1.md` found that
host-boundary acceptance depended on the written scalar expression. This
follow-up starts from clean lane/a commit 7993590; no interrupted edits
were present. The earlier stage-7 restoration remains in that history.

## Interpretation and implementation

Spec §10.3 says: “Where i code gives a value to the host, an `Int` or
`String` member counts as `Int` or `String`, whatever its value set”. It
also says that where the host gives a value, the member “must have all
its values”. Spec §4.2 says each scalar literal type contains only its
value; §4.4 defines union, intersection, and difference as operations on
members. These require the same decision for equal normalized sets.

The resolver now converts a closed boundary expression into the existing
solver algebra, expanding aliases and using the same union, intersection,
and difference smart constructors as inference. The conversion does not
infer unknown variables or holes. Structural members remain representable
while normalizing (for example `Int & {Int | List(Int)}` is Int). Only
normalized ABI members reach the directional host check. The solver's
custom-type variance table is obtained lazily if a parameterized custom
member occurs. Opaque custom types retain their existing ABI.

Service error rows are normalized together, then each resulting member
is checked as incoming. An empty normalized row has no error slots;
whole scalar members have one slot per head in B.2 canonical order.
Restricted incoming members still give E0318/E0320. The no-environment
check also recognizes normalized Never rather than only its named form.
Spec §11.7 explicitly identifies no environment with `! Never`. The converter matches every current HIR type variant explicitly so later tracks must extend it for their own new forms.

All four exact review sources are in the checker regression test
`host_boundaries_depend_on_scalar_sets_not_written_forms`. Independent
expectations are set identities: `{1 | 2} - 2 = 1`, `Int & 1 = 1`,
`{Int - 0} | 0 = Int`, and `String | "error" = String`. Additional cases
exercise aliases, both scalar heads, nested operations, removed foreign
heads, structural members, empty errors, and mixed whole/restricted error
rows. They require acceptance or the exact error-code list, not snapshots.
The executable spec adaptations call real math and Kv providers and check
success and both String/Int error routing at O0 and O2. Separate spec
rejections retain genuinely restricted results and errors.

## Audit of related written-form checks

Every place below was inspected in this follow-up. The search inventory is
`target/t1/continued/review-scalar-search.txt`. This is a code inspection,
with the tests named below as additional evidence, not an exhaustive proof.

| Place | Finding and reason |
|---|---|
| `resolve.rs`: `scalar_abi`, `lit_parts`, `host_type`, `host_part` | Defect fixed: remove the whitelist and use normalized members. Error-recovery exceptions remain for already-reported malformed aliases. |
| `resolve.rs`: `host_fn` errors and `is_never` | Same defect fixed: combine the complete error row first, and recognize normalized empty rows/environments. |
| `resolve.rs`: `external_type`, `host_fns`, `env_type` | Written structure checks retained. §10.1 requires a written function type with no holes/type variables or explicit effects; §9.2 requires one named service type. These are syntax/nominal rules, not claims about scalar membership. Scalar part decisions now delegate to the normalized path. |
| `resolve.rs`: `ty`, alias expansion/substitution, dependency/child visitors | Preserve canonical LitValue leaves; recurse through every set operation. No scalar admissibility rule here. |
| `types/infer.rs`: `conv` and entry-point `host_type`/`host_part` | Already use solver smart constructors and normalized Ty members. Export input checks inspect `values().is_all()`; output values/errors allow restricted scalar sets. |
| `types/infer.rs`: `binary`, `check_places`, `neg_literal` | Equality checks normalized known sets via `known::disjoint`. Place restrictions concern written type variables (§4.4). Unary minus deliberately depends on literal syntax through groups (literal-types decision 5), not on an arbitrary singleton expression. |
| `types/known.rs`: `kconv`, `members`, `disjoint`, `known_bound`, `whole_lits` | Known conversion uses normalized known union/intersection/difference; membership, widening and exclusions inspect ValueSet. Source expression literals supply singleton facts under decision 7. No written-set whitelist. |
| `types/solve.rs`: smart constructors, bound merging and scalar routing | Use ValueSet union/meet/minus/disjoint/subset, not HIR spelling. The new resolver path reuses these operations. |
| `types/pattern.rs`: typed-pattern visit, scalar candidates and narrowing | Converts written tests using `conv`, flattens normalized members and emits All/Only/Except. Narrowing and shared-place checks inspect normalized scalar members. |
| `types/coverage.rs`: `scalar_pat`, usefulness, taken rows and witnesses | Consume ValueSet/coverage patterns, not written annotations. Finite/cofinite and literal-index oracle tests remain unchanged. |
| `types/display.rs`, `types/shown.rs` | Exact scheme trees come from normalized Ty; the full writer retains every member. Limited human text is presentation only. K1's invariant-place condition is unchanged. |
| `query.rs`: `show_type_prec`, `shown_type_parts`, navigation | Written annotation display/navigation intentionally preserve written structure (joint design §4: “Written HIR text”). Literal counting only shortens human hover; no type decision reads it back. |
| `commands/package.rs`: `interface_of`, `TypeShow::ty`, canonical writers | Value schemes use full normalized `scheme_types`; ordinary aliases and constructor fields use the full written path, with canonical LitValue spelling. Joint design §4 explicitly says ordinary aliases/fields “retain the written-type path” and “The package layer may report a type change under its existing policy”. There is no general set-equivalence acceptance check here. Hidden literal members are never truncated. |
| `ilang-wasm/src/lower.rs`: typed patterns, `test_cond`, host signatures | Runtime tests consume validated TestMember All/Only/Except; ABI lowering consumes HostFn/HostSignature. It does not reclassify written scalar types. Ordinary literal-pattern equality is required by §7.1. |

The syntax design's obsolete records-error example is replaced by
`{ x: [1] -> 2 }`, and `{ x: 1 -> 2 }` is identified as a clause block.
The exact governing literal-types §6.1 sentence is quoted beside the
updated subsection, as requested. This resolves the previously reported
stale source; no new controlling-source conflict was found.

## Everyday E0520 costs

Literal-types §8 and spec §6.9 require rejection when the operands' known
value sets are disjoint. These examples were accepted before T1:

- `x = 1` followed by `x == 2`: compare through `equal = (a, b) { a == b }`,
  then `equal(x, 2)`, when a runtime comparison is wanted.
- `status = () { "ok" }` followed by `status() == "error"`: declare
  `status = () -> String { "ok" }`.
- `list.map(["a"], (x) { x == "b" })`: annotate `(x: String)`.

For the concrete inputs and bodies shown, none of the three comparisons
can ever be true. The repairs expose a wider comparison contract; they do
not change those runtime answers. The checker tests require E0520 for all
three original sources. `6/equality_repairs/` requires all three repaired
runtime results to be False at both optimization levels.

The later language question remains open and is not implemented: can rule
13 take a tuple shape from inference without losing principal types?

## Verification record

Before rebuilding, the prior release binary reproduced all four review
failures (three E0318 and one E0320); raw output is in
`target/t1/continued/review-before-fix.json`.
The focused checker suite passes 25 tests. Four normalized-boundary spec
cases pass at O0/O2. Final merged-tree gate results are recorded in
`tests/t1-stages.md` below the previous completion record.

Development failures: an added test initially treated the whole set
`Int & {Int | List(Int)}` as restricted; the expectation was corrected
using §4.4's set identity. A spec import initially used `std/fail` where
the standalone spec harness provides `i/fail`; corrected. The runtime
repair fixture initially used unsupported unary `!`; its assertions now
compare the runtime Bool with False. Its list-map case also needed a package fixture with an explicit std dependency; `i/list` is not a library module. No compiler check or test was
weakened. The first old-syntax run also rejected eight Rust format templates before substitution; the templates now contain valid type-name placeholders and use string substitution, with no scanner exceptions added. Its next run parsed 1,330 snippets with exact expected diagnostics and the same 17 other-track futures. No check was interrupted; no global environment settings changed.

## Final follow-up verification

The final implementation checkpoint is 3718ea0. Repeated
`git merge refs/heads/redesign` calls reported “Already up to date”; the
upstream ref remained 94789e6141d9d2213d3b909b07bcbac9bbcdb02d at final
verification. The follow-up started clean and preserves the previous T1
work. The earlier syntax-note escalation is resolved by this user's
explicit request; no other controlling-source disagreement was found.

The requested whole gate passed. Exact final command, from this lane:

```sh
ILANG_BROWSER_PORT=4181 /home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i ./scripts/dev-verify
```

Exit 0: fmt, Clippy with warnings denied, all workspace Rust tests,
224 spec cases at O0 and O2, 1,465 std tests and 12 bench tests per level,
hello, WASI smoke tests, JavaScript tests, and 51 browser tests. The raw
final log is `target/t1/continued/review-full-gate-final.log`. A preliminary
whole run also completed with exit 0; the final run was repeated after
the mechanical template cleanup and exhaustive HIR dispatch change, so
its results cover the committed final sources.

`cargo test -p ilang-check --test literal_types --locked` passed 25 tests.
The four boundary spec cases and the equality-repair package also passed
separately at O0/O2. All four exact review sources now check with the
rebuilt release binary (exit 0), in `review-after-fix.json` under the same
target report directory. The compatible pre-T1 94789e6 binary accepts all
three original everyday examples (`review-everyday-before.json`); the new
checker tests require E0520 for them, and the runtime repairs give False.

The final old-syntax command is
`direnv exec /home/agent/Programming/i ./scripts/old-syntax-check`: exit 0,
1,330 snippets, exact expected diagnostics, 17 other-track futures and no
T1 future entries. The scanner and its exception data were not changed.
The corpus command `tests/t1/check-all.py` with the compatible pre-T1
binary, the rebuilt release binary, and `--base-ref 94789e6` checks 321
cases: 262 kept, 58 added, zero gained, one lost. That loss remains
`crates/ilang-check/tests/types/spec.i`, `assert(1 == 2)`, required by
literal-types §8/spec §6.9. The original four runtime-fixture rewrites
remain separately counted in the earlier T1 report. There is no new loss.

No golden line changed in this follow-up:
`git diff 7993590 --numstat -- '*.out'` is empty. No regeneration was
needed. The previous per-line rule audit remains in
`tests/t1-golden-rules.md`.

## Optional fresh performance repeat: unqualified

The earlier valid T1 performance gate at 7993590 remains historical
evidence (+8.401% std, all stress budgets and four bound counters passed).
It is not claimed as a new measurement of this follow-up. The requested
whole gate above is green; an extra fresh performance repeat did not
obtain a valid quiet window. Its measurements are not a new performance
pass and are not evidence of a numerical regression either.

All fresh measured attempts used the unchanged aggregate script,
compatible pre-T1 94789e6 substitute binary, CPU 6, one warm-up, and 11
sequential samples per binary. This baseline remains an explicit
substitute for obsolete 1865366/1df2fe0, with provenance in the earlier
report. Every measured attempt failed the unchanged background-CPU limit
of 5%; attempt 1 also missed the +10% std target. Every hard +18% ceiling
passed. The aggregate stopped at std, so fresh stress/counter runs did
not occur in these failed repeats.

| Attempt | std change | Peak background CPU | Numeric limits | Quiet limit |
|---|---:|---:|---|---|
| 1 | +10.231% | 9.301% | fail | fail |
| 2 | +8.223% | 18.014% | pass | fail |
| 3 | +0.494% | 17.379% | pass | fail |
| 5 | -1.439% | 14.061% | pass | fail |
| 6 | +7.325% | 9.380% | pass | fail |
| 7 | +5.174% | 9.879% | pass | fail |
| 8 | +4.114% | 14.661% | pass | fail |
| 9 | +7.754% | 8.681% | pass | fail |
| 10 | +7.298% | 17.307% | pass | fail |
| 11 | -2.906% | 21.669% | pass | fail |
| 12 | +8.142% | 17.139% | pass | fail |
| 13 | +6.404% | 9.051% | pass | fail |
| 14 | +3.948% | 19.142% | pass | fail |
| 15 | +8.942% | 14.871% | pass | fail |

Attempt 4 was only a bounded preflight under one quiet reservation. It
exited 3 after 60 seconds without three consecutive quiet samples; no
benchmark was run. Raw reports remain in
`target/t1/continued/review-performance-N/` and matching `.log` files.
No numeric limit, sample count, or quality threshold was changed. A fresh
qualified performance repeat remains available to the coordinator using
`tests/t1/perf-gate.sh`; it is not presented as a completed fresh gate.

Two optional observer waits were deliberately stopped: the unreserved
ambient observer before it launched a benchmark, and the retry observer
only after attempt 15 had finished and it had no active child checks.
A stop guard first refused to stop the retry observer while its child
was active; that guard took no action. The observer was then paused,
the child completed, and the idle observer was terminated. No benchmark,
implementation test, or external process was interrupted or signalled.
Shared locks were released. Browser port and CPU affinity were local to
the ended commands; no global environment or git configuration changed.
The normal npm/build caches from verification are retained, with no
tracked environment changes. No further implementation work is pending
for the independent review finding.
