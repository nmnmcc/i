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

## Follow-up proof and measurement checks

A later spelling probe used Svc(Extra | Dep) in a callback row and
Svc(Dep | Extra) in its handler. Textual witness matching incorrectly
rejected it. `same_form` now proves variable equality by inclusions in both
directions and compares union/intersection members as sets. Nested rows
inside invariant `Both` views also keep canonical display order. The
regression requires both spellings to have the same accepted scheme.

The first exact standard-project timing showed an approximately 7–9% CPU
regression. Fresh variables now acquire row modes without a graph walk;
constructors with one possible form need no retained obligation; and
coalescing avoids allocations for unbounded variables and small, already
flat bounds. The stack-cycle check still runs before the unbounded-variable
fast path. Genuine recursive-type negatives retain E0505.

The first archived redesign build shared the lane's Cargo target directory.
An identity probe found the K4 diagnostic string in that supposedly base
binary: Cargo reused a workspace artifact. Those baseline comparisons and
timings were discarded. The final baseline uses its own target directory,
recompiles every local crate, contains no K4 diagnostic string, accepts z1,
and rejects q21 with the expected E0505. Third-party dependency artifacts
alone are reused. No shared checkout was changed. An initial isolated build
invocation had the wrong working directory and was stopped; the corrected
isolated build and WAT export completed. A missing Binaryen scratch symlink
and two working-directory mistakes in scratch setup were also corrected.

The raw review generators produced two divergent programs,
plain-44-188 and plain-44-361: both contain exactly
`h0 = () { run(h0) }`, and their entries call it. Both old and new compilers
accepted them, and both optimization levels timed out in each compiler.
The source call rule gives no returning execution here; these are not cast
failures or checker nontermination. The raw results are retained. An
additional bounded corpus replaces only recursive edges of Int helper
functions with a pure Int callback and retains every original source.
This is explicitly a substitute runtime measurement for those recursive
inputs, not evidence that the raw programs terminate. The CLI comparison
reports compiler timeouts and runtime cast failures as failures; it reports
an exact immediate self-call timeout as expected source divergence.

The merged redesign rejects the recursive service timing ring with E0505.
Consequently the ring's before/after comparison uses the pre-follow-up lane
binary at 63e5f3a as a substitute baseline. Standard-project timing uses the
actual merged redesign. Every sample, compiler identity probe and runtime
result is retained under target/k4d-checks.

## Alias and runtime boundary completion

F2 also applies when a function alias has no algebraic operators. Alias
instantiation's early return previously bypassed the parameter modes already
computed from its definition. `instantiate_alias` now marks each actual
argument before applying the template. The environment and error regressions
use concrete Svc(Int) and Svc(Nil) parameters to constrain one row variable;
rule 4 requires equal arguments, so both must give E0515. An earlier test
used unannotated constructors with unconstrained callback results; those can
share a wider result type and were correctly accepted. That expectation was
corrected to test the stated rule with concrete arguments.

The original K4 service, helper, nested-service, logging and local-closure
shapes also run through the CLI with all providers removed. Both optimization
levels must stop with E0522, including the 64-helper chain. These runtime
boundary checks complement the original principal-scheme and query checks;
they do not replace them.

The standard-project profile showed allocation and copying costs in repeated
simplification. Parameter modes and variances now use owned, allocation-free
iterators for the usual small descriptions. Unusual large descriptions still
copy their descriptor, allowing constraints to mutate the solver safely.
Occurrence contexts borrow the immutable type being simplified instead of
cloning neighboring type trees. Handler input occurrences are collected from
the original occurrence counts before those counts are changed. These changes
preserve exactly the same intersection of occurrence contexts and the same
substitution order; only a selected substitution is cloned. This follows
4.7's requirement that simplification never change correctness or behavior.
Callgrind profiles and timing samples are retained separately: instruction
counts are a substitute profile, not elapsed-time measurements. Valgrind
reported its brk-segment limitation and completed both profiles successfully;
it required no environment changes.

## Coalescing failure audit

These are the recovery or skipping paths audited against the first review's
requirement that no coalescing failure be silent:

| Path | Required behavior |
| --- | --- |
| `display::simplify_steps` | Use the solver's bound reductions; mark every irreducible failure. |
| `Checker::generalize` | Report the failure before retaining a raw type for diagnostic recovery. |
| `Checker::entry_point` | Report before returning; a failed scheme cannot skip entry validation successfully. |
| `Checker::run` and `Typed::lines` (`--types`) | Missing or failed schemes are diagnosed; only a diagnosed partial result may omit a line. |
| `Checker::exported_type` | Report before skipping the visibility walk. |
| `display::scheme_names` and `scheme_in` | Propagate `Err`, including a failure in the enclosing scheme. The tuple extraction is guaranteed by constructing a tuple. |
| `display::known`, `known_keep`, `known_shown` | `Top` is recovery with the same failure flag, drained at binding/group/program boundaries. |
| `Checker::check_case` | Its optional scalar-warning detail uses the same flagged coalescer; a failed detail is drained at the group boundary. |
| `Checker::read_shape` and `check_value_places` | T2's raw-type recovery uses the flagged coalescer; the group/program drain rejects success. |
| `LocalTypes::show_in` and `named_types` | Both polarities are validated before a query snapshot leaves the checker. Fallbacks assert that the program was diagnosed. |
| CLI `package::interface_of` | Stop on checker errors; missing text or exported canonical trees are explicit compiler errors, never `_`. |
| Wasm `lower_with_bindings` | Return retained diagnostics before constructing IR, even for a direct caller of the lowering API. |

T2's `completed` returning an unresolved shape is not a coalescing failure.
Its staged transaction either supplies all demanded closed equations or
reports the value-type diagnostic; recording an optional wholly closed shape
is allowed to skip an unresolved shape. Those paths retain T2's transaction
and observation rules.

## Base-accepted programs now rejected

Both the review base (94789e6) and the actual merged redesign (c2f41b7)
accepted every row below. The named comparison runs both base binaries and
the lane at both optimization levels. Base behavior in this table was the
same at opt 0 and opt 2.

| Program | Base behavior | Lane diagnostic and reason |
| --- | --- | --- |
| z1 | Prints `other` instead of `two` | E0515: an annotation hides a handler's Svc(String) same-form obligation. |
| z4 | Prints `other` instead of `two` | E0515: the equivalent annotated handler loses the same Svc obligation. |
| g_inf_SDep | Cast failure | E0515: row comparison inside a type argument changes the required service form. |
| m3 | Cast failure | E0515: the nested row's service arguments must be equal. |
| m4 | Cast failure | E0515: the nested row's service arguments must be equal. |
| r6 | Cast failure | E0515: the row's service arguments must be equal through the helper. |
| b4 | Cast failure | E0522: the entry retains an unprovided service. |
| b7 | Cast failure | E0522: the entry retains an unprovided service. |
| r2 | Cast failure | E0522: the entry retains an unprovided service. |
| r3 | Cast failure | E0522: the entry retains an unprovided service. |
| row_alias | Pure unused-code entry returns normally | E0515: a closed alias contains incompatible S forms. Invalid under F2; no runtime failure claimed for its unused code. |
| alias_kind_env | Pure unused-code entry returns normally | E0515: the function alias makes N an environment row; concrete Svc(Int) and Svc(Nil) cannot inhabit that row. |
| alias_kind_error | Pure unused-code entry returns normally | E0515: the same contract applies to an error-row parameter. |

The review bases already reject x1/x4/x6/x7/x8/g_ann_SDep with E0505.
The lane now gives E0515 for their actual same-form violation. They are not
included in the base-accepted inventory.

## Failed and interrupted development checks

All failures below were resolved before the final gates. Expected negative
checks inside the runtime harness are successful assertions, not gate failures.
Logs live under `target/k4d-checks` unless otherwise stated.

| Checks | Failure or interruption | Resolution |
| --- | --- | --- |
| Initial T1 merge build | Three constructor patterns omitted the literal value-set field. | Keep the T1 field throughout the K4 operations. |
| `named-first`, `named2`, `obligations`, `hidden`, `hidden2`, `trace-w` | Finite field/handler E0505, an output-only row incorrectly retained as an input, and early harness parsing/undefined-helper mistakes. | Separate handler obligations, preserve the correct polarity, and repair the complete test programs. |
| `checker-first` | Replayed mode checks duplicated an existing diagnostic. | Deduplicate identical solver failures. |
| `checker2` | The old unsafe coercion expectation used E0501. | F2 requires row-form E0515; retain rejection. |
| `checker3`, `checker5` | Diagnostic/canonical-order golden differences, including interim unrelated scheme changes. | Restore unrelated schemes; generate only the four documented derived changes. |
| `checker4`, `row-modes` | Boolean call sites remained during the Mode conversion. | Pass the persistent row mode consistently. |
| `row-schemes` | The test helper's tuple was mistaken for a Checked result. | Invoke the checker on its graph. |
| `spec-first` | A constructor with one possible form was treated as a nontrivial hidden restriction. | Prove that obligation universally, without a type-name exception. |
| `clippy-first`, `parameter-iteration` | Collapsible conditions and an owned-iterator lint. | Restructure conditions and make the necessary owned fallback explicit; no lint suppression. |
| `dev-verify-head` | Formatting failed while the next iterator change was being prepared. | Format the final source; repeat the full gate. |
| `delivery-build`, `alias-kind-test` | The initial alias test demanded rejection of freely unifiable callback results. | Use concrete, incompatible annotated arguments; retain the alias mode fix. |
| Initial generator adapter | No entry point because source construction stopped early. | Keep all source-building statements; discard that run. |
| Initial Python audit script | `enum.py` shadowed Python's standard module. | Rename the local audit script; no global Python change. |
| Initial snippet checks | Partial constructor, service, or entry-marker fragments were parsed as complete snippets. | Use complete source templates; keep the snippet checker unchanged. |
| `timing-isolated` | The real redesign rejects the ring with E0505. | Label the pre-follow-up lane as a substitute ring baseline; keep redesign for std. |
| Initial timing runs | Approximately 7–9%, then 3%, then 1.5% std CPU regressions. | Remove repeated row/occurrence allocation and copying; final measurement is separate. |
| Initial archived baseline, `baseline-correct`, interrupted isolated build | Cargo target contamination, scratch working-directory mistakes, missing Binaryen source link, and a canceled cold build. | Discard invalid baseline results; rebuild all local crates in the isolated target and verify compiler identity. |
| Raw generator runtime checks | Two exact source-level recursive helpers time out in both binaries and both optimization levels. | Retain these results as source divergence; also measure the separately labeled bounded corpus. |

No checks or lints were weakened. Temporary trace instrumentation was removed.
No global configuration, PATH, package lock, shared source checkout, or other
lane was changed. Scratch sources, dependencies and logs remain only under
this lane's ignored target directory. Browser dependencies are installed by
the normal dev-verify command.

The final original-helper runtime test initially used a Rust format string
whose placeholders were extracted as an i snippet (`delivery-syntax-first`).
It now substitutes names in a complete i template, preserving exactly the
same 64 helpers. The snippet parser and its expectations are unchanged.
The plain generator sorts its set of used helper names, making seeded source
construction stable across Python hash seeds. The named comparison also
asserts the principal use/fw schemes for all twenty q spellings, from 4.8's
union of the callback row N and service parameter S(N).

One queued quiet measurement was canceled before collecting any samples:
it held slots while the verification command was about to request its nested
snippet-build slot. Only this lane's waiting quiet wrapper and its flock
child were terminated, releasing the locks. The completed quiet measurement
is started after the gates and comparisons finish.

The nested-slot issue also occurred when another lane's quiet measurement
queued behind this lane's last snippet step. The already-completed Rust,
i/JS, spec and WAT checks were retained, but that final snippet step was
interrupted. The queued replacement verification had not started and was
also replaced. The final command runs old-syntax-check outside the outer
heavy wrapper, allowing its own wrapped Cargo build to acquire a slot. No
other lane's process, source, or measurement was stopped or modified.

## Principality and termination argument

The former row bounds represented inclusion by ordinary value-variable
bounds. Following that aggregate back through a service argument could
invent a constructor cycle although the finite solution is N | S(N).
Rows now collect their member requirements in the receiving variable while
retaining equality comparisons on service arguments. The inclusion shortcut
only removes A - H <: A (also through an already proved inclusion), or
A <: A | B. Difference obligations are separate scheme constraints, not
recursive constructor bounds, and are instantiated with the same variable
mapping as the visible type.

Normalization also uses equality proved by both bounds: T <: v <: T fixes
v to T. It never discards N <: S(N) or S(N) <: N. Each rewrite either removes
an entailed bound or fixes one variable to an already present exact type;
there are no new variables. On a fixed bound component this reaches a finite
fixed point. Row modes have three ordered states and only increase; the
parameter-kind pass likewise terminates after finitely many upgrades.
Constraints retain their existing cache, now including the mode. Handler
restrictions propagate over finite member-variable reachability and are
copied under the existing level/generalization discipline. Genuine
constructor cycles still fail before a successful scheme leaves the checker.
Coalescing expands a finite graph after alias SCC collapse; completed
expansions are memoized by representative, polarity and row mode. A cycle
failure is never memoized as a successful type.

These are checks of interpretation, separate from the implementation tests:
4.5 rules 2 and 3 require those inclusions for every assignment; 4.5's type
equality sentence requires mutual inclusion; 4.7 requires principal schemes
and rejects actual self-containing types. F1 requires the restriction to stay
in the scheme, and F2 requires the row mode everywhere. The old scheme
rebuild lost a real obligation and the old constraint-local flag lost a real
comparison rule; neither was a valid simplification. F3's explicit finite
handler spellings and 4.8's field/call rules require the same acceptance as
pattern access. The retained restriction is checked at every instance, even
when an annotation shows the wider result row.

## Functions changed for merge planning

This inventory compares K4 with merged redesign c2f41b7, excluding T1/T2
changes inherited by the merges. Test functions are covered by their test
files above.

| File | Functions |
| --- | --- |
| `types/solve.rs` | `Solver::new`, `for_closed_types`, `fresh_at`, `variances`, `fail`, `constrain`, `constrain_env`, `rows`, `mark_row`, `mark_mode`, `is_row`, `mode`, `constrain_row`, `has_obligations`, `handle`, `same_form`, `hidden_handlers` (and its `parts` walker), `constrain_in`, `constrain_by`, `add_bound`, `compare`, `join`, `route`, `diff_left`, `extrude`, `freshen`, `env_union`, `union_mode`, `merge`, `merge_arg`, `diff`, `env_diff`, `diff_mode`, `normalize_mode`, `diff_bound`, `diff_bound_mode`, `occurs`, `normalize_bounds`, `normalize_type`, `normalized`, `occurs_from`; `edge_vars`, `bound_vars`, `env_vars`, `builtin_variance`, `builtin_variance_values`; `ParamModes::{repeated,from_slice,next,size_hint}`. |
| `types/infer.rs` | `Checker::new`, `run`, `group`, `entry_point`, `exported_type`, `check_occurs`, `report_coalescing`, `add_err`, `add_env`, `pop_effects`, `conv`, `fresh_args`, `function`, `local_binding`, `generalize`, `rebuild`; `row_parameters` and its `walk`. |
| `types/display.rs` | `scheme_in`, `scheme_names`, `simplify_steps`; `Coalescer::go`; `Norm::{norm,env,union_in,step,walk}`; `record`. |
| `types/alias.rs` | `Checker::instantiate_alias`; `normalize`. |
| `types/known.rs` | `Checker::ctor_known` (consume the variance iterator). |
| `types/value_types.rs` | `closed_equal`, `normalize_closed`, `Checker::closed_written` (preserve row metadata in closed-type scratch solvers). |
| `types/mod.rs` | `LocalTypes::{show_in,named_types}`, `Typed::{coalescing_errors,lines}` and their diagnostic-state fields. |
| `ilang-cli/src/commands/package.rs` | `interface_of` (explicit missing-scheme/canonical-tree errors). |
| `ilang-wasm/src/lower.rs` | `lower_with_bindings` (retained diagnostic guard). |

No type or library name participates in the solver fixes. The built-in
Continuation/Boundary parameter roles come from their language-defined
function/error/environment positions. The existing type-parameter variance
rules continue to apply to value arguments and to the row as a whole.

The q regression loader now requires every fixture to exist (`unwrap`);
a missing file cannot silently skip a named program. The independent named
runner already required all twenty files. This test-only tightening is
rechecked with the CLI K4 test after the full gate.

The first completed quiet final run was effectively flat on std (median CPU
0.322220 s before and 0.323087 s after, +0.27%; paired-difference bootstrap
interval included zero). It did not give a clear margin for the strict
not-worse criterion. A final allocation audit removed the temporary clone
of every bound before coalescing copied it into the result. The solver is
immutable during coalescing, so iterating those bounds by reference preserves
all constraints, traversal order and cycle checks. Variance lookup now skips
argumentless constructors, and the row-parameter fixed point skips definitions
without parameters: neither can contribute a parameter property. The final
measurement and full gate are repeated for these changes.

The CLI boundary tests additionally run the first review's exact annotated
`@main main(N)` reproducer and its unannotated twin. Both must give E0522.
Replacing the call with Nil must succeed at both optimization levels: 4.8
rule 4 says that merely creating the unused function needs no service.
