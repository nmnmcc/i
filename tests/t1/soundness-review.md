# T1 soundness review

This is a rule-led self-review, separate from generated expectations. The
programs below were chosen for what would go wrong at run time if a
constraint or effect were lost. No generated golden is used as the oracle.

| Rule and required behavior | Independent probe |
|---|---|
| Literal-types §5.3: “a literal at any place below the top ... always goes to useful()”. Replacing only part of a tuple/list row must not exclude a value from another clause's binder. | `7/t1_rf_tuple`, `t1_rf_list`, `t1_rf_type_column` must give E0501. Each main would pass `"a"` to a helper accepting only String minus `"a"`; that helper has actual exhaustive clauses, rather than ignoring the argument. |
| Spec §7.4: “A clause with a guard is not counted.” | `7/t1_rf_guard` must give E0501 for a False guard. The separate literal_guard_index pair runs the later row and rejects a match covered only by a True guard. |
| Literal-types §5.3: “A candidate is taken when the catch-all, replaced by a pattern that matches exactly that candidate, is not useful against the earlier rows.” | `7/t1_rf_covering_row` must run: `("a", _)` covers that value for the entire other column; `"b"` and `"c"` reach the strict cofinite helper and return 1 and 2. |
| Joint J5: “Nested literal failures may still be held, but are dropped only if the same match actually reports E0516; otherwise flush E0501.” An opaque test does not test a Box's field. | `7/t1_rf_opaque_literal` must give E0501. The nominal test would take Box(`"b"`) as Box(`"a"`) if the held field failure disappeared; the strict helper's fitted field would then hit unreachable. |
| Joint J5: “A one-clause block with choices is fitted and can need Assert.” Spec §7.3: “A literal pattern can not fail for its literal type.” | `7/t1_rf_unknown_return` must stop through its own Assert handler for a callback returning `"b"`; `7/t1_rf_finite_fitted` must run with a written no-Assert result and both possible inputs 0 and 1. |
| Decision 9: “Where the host gives a value ... must have all its values.” | `10.1/t1_rf_external_result` must give E0318 and `9.7/t1_rf_host_result` E0320. These use actual matching test providers: add(2,3) returns 5; Kv.get(`"name"`) returns `"i"`. A false acceptance would fall into unreachable finite matches. |

The focused command `cargo test -p ilang-cli --test spec --locked -- t1_rf`
passes all ten probes at O0 and O2. The first run passed seven; the other
three had a tuple clause immediately after a numeric body on the same
line, which was parsed as a call and gave E0106. Separating the clauses
by line breaks fixes the fixtures. No compiler rule or check was weakened.

The wider review checks the scalar algebra's empty/canonical/cofinite
cases, deferred routing through variables, environment equality,
field/tuple bounds at every polarity, and runtime tag/equality ordering.
The model/solver tests and existing literal_type_test cases supply these
checks. The latter include wrong heads, boxed positive/negative Ints,
large Ints and String complements at both optimization levels. Hidden
callback returns, errors, setters, environments, returned functions and
unknown parts have the stage-7/8 accepted and rejected programs. Full
interface publication tests cover a changed literal beyond the shortened
hover's tenth item. K1's invariant-variable regressions remain in the
full gate. No new soundness hole was found in these probes.

The std-1b §5 guarded-list regression K2 is fixed by T1's place preparation.
An unguarded list rest covers later item places, also places introduced by
another clause; a guarded rest supplies no such slack. The exact reduced
probe and a runtime case exercise the guarded `[37, a, b, ..tail]` followed
by `[37, .._]` and `[x, ..tail]` at O0/O2. No std URI workaround was edited.

The accepted note's existing limitation for callbacks that change their
entire constructor family (literal-types §13 question 8) is not a claim
of this scalar widening implementation. No broad change to known shapes
or principal inference was made under the performance work.

The coordinator's required open language question stays for a later track:
can rule 13 take a tuple shape from inference without losing principal
types? T1 does not do that. The amended None-accumulator acceptance row has
`Some(p: (Int, Int))`; the reduced unannotated program remains an exact
E0506 rejection test.
