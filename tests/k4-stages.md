# K4 second review follow-up

The controlling rulings are the user's second K4 review request and the
review criteria in `/tmp/review-k4d/criteria.md` (2026-11-11).

The F2 ruling, quoted verbatim:

> a type parameter that is used as an environment (after `!`) or as an error row is a row, and rows are compared by rule 4 wherever they occur — service forms with the same constructor must have equal arguments — also when the row is the argument of another type constructor. Rule 6's variance applies to the row as a whole (a row with fewer services is still below one with more), never inside a service's arguments.

## Interpretation checks

- F1: "the obligation of an environment statement is part of the scheme."
  A handler restriction is stored separately from ordinary type bounds,
  copied during instantiation and extrusion, and checked against concrete
  forms. Restrictions propagate to member rows before generalization; a
  reverse inclusion link cannot be their only owner. A constructor without
  parameters has just one form, so its obligation is universally true. Generalization retains bounded schemes carrying restrictions.
  A shown scheme that hides a restriction without a difference or a
  correlated row form is rejected with E0515, naming the service.
- Spec 4.5 rule 3: "A - B <: A." Recording that inclusion, also through
  already established variable inclusions, must not invent a recursive
  type. Removing it preserves solutions; the handler restriction remains.
- Spec 4.5 rule 2: "A <: A | B." Existing redundant-union reductions stay.
  Equal-bound reduction uses "Two types are equal when each is a subtype
  of the other." None of these reductions erases a handler restriction.
- F2 and spec 4.5 rule 6 determine row parameter modes by a finite monotone
  fixed point over definitions and aliases. Each solver variable stores
  its mode. A later mode discovery rechecks existing bounds. Modes survive
  freshening, extrusion, generalization, coalescing, and known field bounds.
  Rule 4 still writes `E <: F`; rule 8's scalar literal inclusion continues
  to apply to errors. Service environments additionally require equal
  member value sets, as rule 4 explicitly requires for N and M.
- Spec 4.8 rule 13 obtains fields from corresponding bound forms; rules 4,
  5 and 9 retain callback requirements and remove only supplied services.
  F3's field and pattern spellings therefore have the same finite types.
- Spec 4.7 requires principal inference and says simplification "never
  changes which programs are correct or what they do." Coalescing memoizes
  completed expansions by variable representative, polarity and row mode.
  Alias components already have one representative; constructor cycles
  are diagnosed. Flattening removes only duplicate union/intersection
  paths. These operations neither add constraints nor fresh variables.
- F5 and union's set meaning require a canonical displayed row. Members
  are sorted by type name before naming variables; variable members follow
  their occurrence in those sorted service forms. Permuting mutual
  declarations is tested, including independent row parameters.

## Implementation checks

The original review files are copied verbatim to
`crates/ilang-cli/tests/k4/`. `cargo test -p ilang-cli --test k4 --locked`
checks every named case and invokes `run --opt 0` and `run --opt 2`.
For unsafe cases, run must stop with a checker diagnostic; successful cases
assert the expected value. The source for these expectations is 4.8's call
and environment-statement rules: the installed pure S(Never) returns 1.
In r4, the first k call reads Dep(2) twice (four); after replacement, k and
fr read the pure callback three times (three).

`crates/ilang-check/tests/row_schemes.rs` independently checks F1 at the
definition, and F5 across declaration permutations. Existing recursive-type
negatives still require E0505. The unsafe Sink-to-Consumer coercion now
requires E0515 under the F2 row ruling, rather than ordinary E0501.

The reproducible generated-program comparison is `tests/k4/compare.py`.
Its three deterministic source generators are adapted from the reviewer's
`gen.py`, `mut2.py`, and `plain.py`; they keep source construction, discard
the original hardcoded scratch paths, and run accepted programs at both
optimization levels with both binaries.

## Development checks and incidents

- The initial T1 merge needed both pending-pattern checks and K4 diagnostics.
  Three automatically merged constructor patterns also needed the new
  literal value-set field; fixed in the next integration commit.
- Early named tests reproduced the finite-handler E0505 failures. Separating
  obligations and removing proven difference inclusions fixed them.
- Interim broad tests caught loss of scalar error inclusion when all rows
  shared the environment mode; persistent Value/Row/Env modes preserve it.
- The first profiling run attributed 18.6 seconds of a 100-function ring
  to coalescing, against about 0.6 seconds in bound addition (inclusive
  timings overlap). Instrumentation was removed after capturing the log.
- Early test-harness errors included an incorrectly parsed equality/pipe,
  an undefined stop helper, and constructor fragments misidentified by the
  snippet checker. These were corrected without weakening the checks.
- The first generator adapter stopped general-program construction at its
  initial HEAD assignment, producing no entry point; its runtime checks
  failed. The adapter now retains all source-building statements. That
  failed run is not evidence about compiler behavior.
- Temporary profiling and baseline source copies live only under this
  lane's ignored target directory. No shared checkout or configuration was
  changed.

Final gate results, the base-accepted/lane-rejected inventory, and precise
function ownership are recorded after the final redesign merge.
