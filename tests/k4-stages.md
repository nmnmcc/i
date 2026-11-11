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

Both the review base (94789e6) and the actual merged redesign (82375c0)
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
| direct_row | Pure unused-code entry returns normally | E0515: a directly written row argument contains incompatible S forms. |
| effect_row_alias | Pure unused-code entry returns normally | E0515: using the closed alias as a function environment must retain its row comparison. |
| latent_row | Pure unused-code entry returns normally | E0515: instantiating the returned callback's row union cannot combine S(Int) and S(String), even when the callback is unused. |

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

This inventory compares K4 with merged redesign 82375c0, excluding T1/T2/W0503
changes inherited by the merges. Test functions are covered by their test
files above.

| File | Functions |
| --- | --- |
| `types/solve.rs` | `Solver::new`, `for_closed_types`, `fresh_at`, `variances`, `fail`, `constrain`, `constrain_env`, `rows`, `mark_row`, `mark_mode`, `is_row`, `mode`, `constrain_row`, `has_obligations`, `handle`, `same_form`, `same_form_in`, `hidden_handlers` (and its `parts` walker), `constrain_in`, `constrain_by`, `add_bound`, `compare`, `join`, `route`, `diff_left`, `extrude`, `freshen`, `env_union`, `union`, `union_mode`, `inter`, `merge`, `merge_arg`, `diff`, `env_diff`, `diff_mode`, `normalize_mode`, `diff_bound`, `diff_bound_mode`, `occurs`, `normalize_bounds`, `normalize_type`, `normalized`, `occurs_from`; `edge_vars`, `bound_vars`, `env_vars`, `builtin_variance`, `builtin_variance_values`; `ParamModes::{repeated,from_slice,next,size_hint}`. |
| `types/infer.rs` | `Checker::new`, `run`, `group`, `entry_point`, `exported_type`, `check_occurs`, `report_coalescing`, `add_err`, `add_env`, `pop_effects`, `conv`, `conv_mode`, `fresh_args`, `function`, `local_binding`, `generalize`, `rebuild`; `row_parameters` and its `walk`. |
| `types/display.rs` | `scheme_in`, `scheme_names`, `simplify_steps`, `cycles`; `Coalescer::go`; `Norm::{norm,env,union_in,step,walk}`; `record`, `hide`. |
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

A final F3 probe found a correct concrete handler annotation still rejected:
`! Svc(_) = svc` with svc: Svc(Int), and a body row N | Svc(Int). The
hole has matching Int lower and upper bounds. `same_form_in` now proves
such equalities from both directions of the existing bounds, including
Never's implicit lower bound and Top's implicit upper bound. It never uses
a lower bound alone. A set of variables currently being expanded makes this
proof finite even on a constructor cycle; a dedicated solver test checks
the guard and confirms the actual occurs check still rejects that cycle.
The checker and CLI compare explicit/holed handlers with annotated/inferred
results; the callback literally returns 6, so the call rule requires 6.
The named runtime corpus gains `closed_handler_hole` (75 cases in total).

Redesign moved to 1a8fee2 while checks ran. Only the two prior-art documents
changed from c2f41b7; compiler, library and test sources are byte-identical.
The lane merged those documents and the isolated baseline tree was refreshed
with exactly those additions. A baseline build and WAT export are rerun there.
The equality proof was edited while a queued gate started; that gate stopped
at formatting (`merged-final-gates`). The proof is formatted and the complete
gate is repeated after its targeted checks. The pending old-binary quiet
measurement was stopped for this new correctness fix; no result from that
interrupted measurement is used as final evidence.

The first exact-hole regression then exposed a display identity: after an
H-only variable was hidden as Top, an inferred result still showed
Svc(_ & Int), whereas the explicit handler showed Svc(Int). The exact-scheme
assertion was retained. `hide` now removes Top from intersections after hiding
those variables, using the intersection identity; neither the scheme's
restriction nor a solver bound is removed. The first `proof-final-build` attempt failed that assertion; the
implementation was corrected rather than accepting the noncanonical expected
string. A queued duplicate (`proof-delivery-build`) was superseded before it
started; the complete targeted run is `final-proof-build`.


The F2 audit also checked the direct spelling T(S(Never) | S(Dep)). It
was incorrectly accepted as T(S(Dep)): written-type conversion used value
variance before the enclosing constructor identified its parameter as a row.
`conv_mode` now passes the already determined parameter/effect mode before
converting algebra. Alias instantiation receives the caller's mode too.
This covers direct type arguments, function error/environment annotations,
environment parameters/statements, and alias expansion. Row unions,
intersections and differences use the same solver normalizer; ordinary
value unions keep rule 6. The two new unused-code regressions (`direct_row`
and `effect_row_alias`) must fail E0515 at check and both run optimization
levels. Their base mains simply return Nil; no runtime failure is claimed
for unused code. The named corpus now has 77 cases. The previous queued
quiet run was stopped for this correctness change and is not final evidence.


The last common-path optimization skips SCC construction when fewer than two
variables were reached: `cycles` only returns classes with multiple variables,
so its result is necessarily empty. The coalescer's constructor-cycle stack
check is unchanged. An occurrence pass with no variables cannot select a
substitution, so it also returns immediately. These avoid unnecessary work
without changing any inferred bound or type.


Row algebra also keeps its mode while schemes are freshened or extruded.
An inferred merge returning T(N | M) previously lost the union's collector
at generalization. Supplying S(Int) and S(String) then left an invalid latent
callback type even when the result was unused. Freshening and extrusion
rebuild the row collector for the copied variables, so both later forms are
compared. `latent_row` checks this unused result; `latent_row_same` supplies
one S(Int), invokes the returned callback and succeeds. Rule 9.2 requires
the former rejection and permits the latter shared form.

All function/constructor row-field merges now use the same mode-aware algebra
entry points. In particular, merging function error rows uses scalar set
union under rule 8, rather than environment value-set equality. The
`error_row_values` runtime regression selects functions raising 1 and 2;
9.3 requires their reified errors to be Error(1) and Error(2). The named corpus
has 80 cases. The pending generated files are discarded only to merge the
new W0503 redesign change, then regenerated with the final compiler.


The scalar-error runtime fixture initially used the wrong constructor name
Err. Its check failed with E0201 (`algebra-final-build`); §9.3 and i/fail
specify Error. The fixture now checks Error(1) and Error(2), preserving both
literal expectations. No implementation or diagnostic check was relaxed.


The final redesign merge is 82375c0 (W0503), integrated by 78f942e. Its
known-disjoint-comparison warnings, literal/value-type changes and generated
expectations are retained. The isolated baseline tree was refreshed with
exactly its changed paths; 1,020 compiler, library, spec-test and build files
were compared byte for byte with the merged revision, with no mismatches
(`baseline-source-82375.json`). Its release build and WAT export use the
separate baseline Cargo target, preserving the contamination fix above.

The four K4 golden changes are generated, not hand edited:
`entry_checks/expected.out` sorts the main row and adds E0515 for unequal
scalar arguments inside a row member in an already negative program;
`environments.out` and `spec.out` sort rows; `soundness.out` names the complete
Reader service forms in two diagnostics. F5 requires the canonical display.
The later F2 ruling requires equal arguments wherever the row occurs and
supersedes the original request to freeze all existing golden text. No
previously accepted golden program becomes rejected.


## Final verification on merged redesign 82375c0

The compiler source is frozen at 78f942e for these checks. Subsequent commits
record this evidence and generate expectations only. The initial lane was
clean at 25c569e; all earlier K4 work was retained.

Commands run from this lane, with H denoting
`/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i`:

- `H env UPDATE=1 cargo test -p ilang-check --test types --locked` generates
  exactly the four files listed above.
- `H env ILANG_BROWSER_PORT=4185 ./scripts/dev-verify` passes formatting,
  Clippy with warnings denied, the workspace Rust tests, 1,465 std tests,
  two hello tests and 12 benchmark tests at each optimization level, WASI
  smoke tests, the JS tests and all 51 browser tests.
- `H cargo test -p ilang-cli --test spec --locked` passes all 242 cases at
  both optimization levels.
- `H python3 tests/k4/review.py` with the three compiler paths and output
  directory recorded in `comparisons-82375.log` passes all 80 named cases,
  including check/types and both run optimization levels in every compiler.
  The base-accepted/rejected table above contains all 16 changed negatives.
- `H python3 tests/k4/compare.py --count 400 --seed 44` was run both raw
  against the review base and with `--bound-recursion` against merged
  redesign, using `--runtime-timeout 5`. Each corpus has 1,200 programs:
  991 rejected by both, 185 newly accepted, 24 accepted by both, no newly
  rejected program and no unexpected runtime failure. The raw lane runs
  have 414 successes and four expected timeouts (two source programs at
  both optimization levels); the base has 44 successes and the same four
  timeouts. The bounded substitute changes 94 sources, retaining originals,
  and has 418 successful lane runs and 48 successful base runs.

The three ordinary project type exports contain exactly 2,197 std, five
hello and 23 benchmark bindings in both compilers. Only three std row orders
change: list_test.collect, sequence_test.observe and
string_additions_test.observe. Each change permutes a union's members;
§4.4 explicitly says “Order has no effect.” There are no added or missing
bindings (`types-82375-comparison.json`).

The exact commands, generated sources, stderr/stdout and per-compiler exit
statuses are retained in this lane's ignored `target/k4d-checks`, in
`delivery-82375-gates.log`, `baseline-82375.log`, `comparisons-82375.log`,
`raw-82375.log`, `named-82375/results.json`, the two generated-82375 output
directories and `generator-82375-summary.json`.


`direnv exec /home/agent/Programming/i ./scripts/old-syntax-check` passes
1,762 extracted literals/blocks with exact expected diagnostics and 17
future-track entries. It runs after the outer heavy wrapper has released its
slot; its own Cargo step still uses the required heavy wrapper.

`H env WAT_DUMP=<dir> cargo test -p ilang-cli --test wat_dump --locked`
passes in both isolated baseline and lane. All 906 baseline export records
are byte-identical, including all 577 actual WAT modules; 329 records contain
expected error output. There are no missing or changed records. The only
four additions are the ilang/wasi, optimized/unoptimized exports of the new
generic_service_rows spec case (`wat-82375-comparison.json`). The release
binary used for every final comparison matches the gate's release binary
byte for byte.


The final quiet measurement passes the std not-worse criterion at the
measurement's resolution. It uses 60 alternating, CPU-pinned pairs against
actual merged redesign, after all other own gates and comparisons finish.
Median wall time is 0.311437 -> 0.311232 seconds; median child CPU time is
0.3057525 -> 0.3057615 seconds. The nine-microsecond CPU difference is flat
against millisecond-scale variation. Paired median differences are -2.407 ms
wall and -2.282 ms CPU; 10,000 paired bootstrap resamples (seed 44) give
95% intervals [-5.513, 1.030] ms and [-5.127, 0.750] ms respectively. Both
include zero: no speedup is claimed for std. Raw samples and the derived
intervals are `timing-82375-quiet/results.json` and
`timing-82375-std-interval.json`.

Command: `/home/agent/Programming/i/.rd/cache/quiet direnv exec
/home/agent/Programming/i python3 tests/k4/perf.py --pairs 60`, with
`--before` i-redesign-82375, `--after` i-delivery-82375, and `--rings-before`
i-before at 63e5f3a. The quiet wrapper takes all three heavy slots; waiting
for QA's active corpus did not interrupt or modify its work.

The ring figures are the explicitly labeled substitute comparison described
above, three alternating pairs per size. Median seconds:

| Functions | Wall before | Wall after | CPU before | CPU after |
| --- | ---: | ---: | ---: | ---: |
| 25 | 0.047367 | 0.012836 | 0.045198 | 0.011440 |
| 50 | 0.518331 | 0.041652 | 0.510706 | 0.040085 |
| 100 | 8.615122 | 0.362887 | 8.495138 | 0.354910 |
| 150 | 41.217351 | 1.585129 | 40.664603 | 1.561601 |

The cause was repeated expansion of the same coalescing paths and repeated
type-tree/occurrence copying during simplification. Completed-expansion
memoization and borrowing remove that redundant work while keeping cycle
checks, polarity, row mode and substitution order. The 100- and 150-function
cases improve about 24x and 26x. The original recursive-type negative tests
still require E0505; the dedicated exact-bound proof test also stops at a
constructor cycle.

All done criteria are met on these measurements: F1 and F2 reject their
unsafe definitions/instances with service-naming diagnostics, F3's finite
spellings and correct annotations agree, F4's owned coalescing bottleneck is
fixed, F5 is canonical, and first-review coalescing/entry/interface/lowering
regressions remain covered. The final redesign ref is still 82375c0.
The only source disagreement was the earlier golden-text freeze versus the
later F2/F5 rulings, resolved in favor of those rulings and documented above.
All failed/interrupted checks and temporary environment incidents are listed
in this record; discarded measurements are not final evidence. Trace code
is removed, normal per-command environment variables have expired, no
shared checkout/configuration was changed, and ignored scratch evidence and
normal dev-verify browser dependencies are retained in this lane.


## Escalations and handoff

The compiler follow-up is complete. Ledger entries 156 and 159 assign the
five waiting std/log probes to promotion after K4 lands, in std/log.i,
std/log_test.i and docs/design/std-2.md. Those files are outside this lane's
owned implementation paths and are unchanged here. The coordinator/Std-2
owner must promote them after integration. No other source work is left in
this lane. The final commit contains only generated expectations and is
named “Regenerate derived files.”


## QA integration follow-up

Started with a clean lane at 9695b32a and retained all K4 work. Redesign
d655684b (QA) merged cleanly as 60ab7b64. Its 1,855-project corpus is the
permanent home of review inputs; the former K4 fixture directory and runners
are consolidated into it. This follow-up is authorized to change the QA
registry, requirements and tooling as well as K4's existing owned paths.

The initial full corpus comparison intentionally used the saved expectations
and failed on changed outputs and unexpected passes. Its differences are
retained under target/k4d-checks/qa-initial-differences.json. The first real
non-K4 regression was review-t1/error_rows: subtracting a previously handled
scalar literal from an error-row variable recorded a service equality between
“a” and “b”. Solver::handle now skips scalar same-form obligations for Mode::Row
while retaining them for Mode::Env. Source: §4.4 says scalar difference removes
values; §4.5 rule 8 says scalar subtype follows value-set inclusion, and the
F2 amendment explicitly retains rule 8 for E <: F. Named service forms still
retain their argument-equality obligation. The unchanged T1 fixture passes
check/types and run at both optimization levels, and coalescing, generic-service
and row-scheme tests pass (qa-error-row-fix.log). Temporary backtrace tracing
was removed before the successful checks.


The migration command preserves 103 former runtime assertions in 101 distinct
permanent projects, reuses 21 existing projects after canonical formatting,
and adds 80 distinct entry/annotation variants. Principal use/fw fragments
for every q21–q40 spelling are explicit requirements, including both original
review copies and the runnable entry variants. Original review bytes remain
unchanged. The old fixture copies, Rust corpus runner and three Python K4
runners are removed; focused checker unit tests remain. The review runner
now also calls run O0/O2 for source-rejected entry requirements and verifies
that all three commands reject before execution. Those additional phases
are a harness change, recorded separately from compiler behavior changes.

Promotion commits identify the finding and enumerate each promoted case.
Semantic requirements are promoted individually. The generated observations
and known-failure report are regenerated together in the final derived-files
commit, following the workspace's explicit generation rule.


## Comparison with the type-inference references

The requested note is absent from merged redesign d655684b despite the ledger
calling it landed. I read docs/design/prior-art-types.md at immutable commit
3beae77d706433c56f7a99b3a1f2a927f1119a6c (also the unchanged note-pa-types
worktree), including §§1–3, 7.2, 8 and the numbered handoff recommendations.
In its priority table, 1 and 2 are obligations/row roles, 3 is advisory
equality (already inherited as W0503), and 8 is coverage-witness validation.
Finite-bound normalization is priority 6. The four mechanisms specifically
requested in this follow-up are compared below without renumbering that table.
The cited code was read directly under .rd/ref, never edited, built or run.
The comparisons are source-based; they do not claim a transferred theorem.

### Handler obligations kept in schemes

GHC/compiler/GHC/Tc/Solver.hs, simplifyInfer (944–1030), solves the captured
wanted constraints, chooses quantified variables and predicates through
decideQuantification, creates evidence variables for the chosen predicates,
and calls emitResidualConstraints. That function (1044 onward) partitions
constraints whose coercion variables must remain outside from the residual
inner wanted set, then emits the latter in an implication with quantified
skolems and given evidence. An obligation absent from the visible result
type is still owned and discharged. The empty-wanted fast path is explicit.

K4 uses that lifetime idea, not GHC's quantifier/evidence algorithm. VarState
keeps handled forms separately from ordinary bounds; generalize retains the
bounded scheme when it has obligations, and freshen/extrude copy them with
the same map as visible variables. hidden_handlers checks that an annotation
is valid for every permitted instance. This is narrower than a general
qualified-scheme language: an annotation hiding the restriction in x1/z1 is
rejected, rather than acquiring a new implicit surface predicate. A GHC-style
qualified scheme could state such a premise explicitly; i has no such written
constraint syntax, and F1 explicitly requires an annotation omitting the
difference to prove the obligation for every instance. No additional accepted
i case follows from GHC's greater expressiveness.

A concrete language difference is GHC's own “Inferring principal types” note
(867): f x = show [x] is assigned Show a => a -> String, whereas a caller
with only Show [a] cannot use f even though it can use the inlined body.
That is not an acceptable inference rule for i (§4.7). GHC also handles
classes, higher-rank/partial signatures and coercion evidence that K4 does not.
Its explicit residual-constraint container is a more general representation
than K4's service-form list, but importing it would add machinery and language
features rather than fix a demonstrated missing i case.

### Row mode on variables and constructor parameters

Koka/src/Type/Unify.hs, unifyEffect (375), extracts ordered labels, calls
unifyLabels (419), extends missing tails and unifies them. Matching labels
call unify on the entire label including its arguments; the remaining lists
are substituted before continuing. unify recursively traverses ordinary
applications (257), so nesting an effect row under another constructor does
not switch its rule. unifyTVar (333) checks the variable kind before binding.
The row role is structural/kinded rather than an accidental caller flag.

K4 has the same equality-of-matching-label-arguments idea. Its inferred
Value/Row/Env roles belong to variables and formal type parameters; a finite
fixed point discovers formal roles, and an upgrade rechecks older bounds.
conv_mode supplies the role before written algebra can merge constructors;
freshen, extrusion, alias expansion and coalescing retain it. Koka's explicit
kinded representation is simpler at each unification call. i has no explicit
effect-kind declarations, so replacing the fixed point with that representation
would require an additional language design. The distinguishing i cases are
g_inf_SDep and direct_row: nesting the row cannot make its S arguments covariant.

Koka handles duplicate effect extensions and distinct rigid heap-scope labels
(the unmatched-label branch of unifyLabels). i intentionally has idempotent
union and one form per service constructor; S(Dep) | S(Dep) is S(Dep), not
two handler layers. Koka's labels also are not i's arbitrary scalar errors.
Applying label equality to scalar error subtraction caused the real
review-t1/error_rows regression above. The implemented correction follows
i's §4.4 and rule 8; service forms still follow rule 4.

### Entailed-bound removal before the occurs decision

Simple-sub/shared/src/main/scala/simplesub/TypeSimplifier.scala, compactType
(49), tracks polar variables in process and a separate parent chain. It
returns an empty bound for a repeated variable still in that chain, removing
a pure link cycle. Under Function/Record it resets parents: a later recurrence
then gets a recursive binder in CompactTypeScheme.recVars. Its later
canonicalizeType explicitly notes the powerset-style worst case.

The common idea is to distinguish a cyclic graph from structural recursion.
K4 additionally proves i's redundant inclusions before the finite-type check:
A <: A | B, A - H <: A, and exact types proved by both bounds. It removes
only entailed bounds, preserving handled forms separately. For example the
constraint graph N <: A and A <: N | S(N) admits the finite A = N; it is not
the equation A = N | S(N). q21 and its annotated/unannotated field/pattern
variants are runnable programs distinguishing that finite case from the old
occurs rejection. The same reductions are used when coalescing retries.

The reference's cycle treatment is simpler and more permissive because it
accepts regular recursive types. Its self-application term fun x -> x x can
have a recursive domain type; the corresponding i function requires E0505.
i's spec forbids that structural self-containment. Replacing an active i proof
with a recursive binder would change the language, not complete this fix.
No broader entailed-bound rule justified by these reads was found missing
from a demonstrated accepted-fragment i program.

### Coalescing, simplification and invariant sharing

MLstruct/shared/src/main/scala/mlscript/TypeSimplifier.scala, simplifyType
(318), counts occurrences at both polarities for invariant positions and
records occursInvariantly. Its analysis and transform agree that a field
whose lower and upper types are identical is invariant. transform (613)
refuses normal polar bound inlining for such a variable, allocates a single
renewal per old variable and copies the relevant bounds. Co-occurrence
unification also merges the bounds and intersects opposite-polarity
co-occurrence sets, rather than just renaming printed variables.

MLstruct's ConstraintSolver.scala, freshenAbove (549), installs the fresh
variable mapping before copying both bound lists, preserving sharing even
when those lists are recursive. extrude (495) caches by variable and polarity,
reverses function inputs, and uses TypeRange for invariant arguments. K4 uses
the same shared-map and polar-level ideas. Its invariant traversal visits
both polarities even after an equal Both has collapsed, and generalize keeps
a raw bounded graph when Both or a handler restriction remains. Completed
coalescing expansions are memoized by representative, polarity and row mode.
An active constructor cycle is a diagnostic, not a completed cache result.
The borrowed occurrence contexts still compute the same intersections; only
a selected substitution is copied.

MLstruct handles full negation, recursive types, TypeRange intervals and
record lower/upper fields; i's current language does not. For example its
TypeRef with an invariant parameter can expose an interval for independently
extruded polar bounds, while i retains Both internally and requires the
service's equal argument at use. The distinguishing current i regressions
are alias_kind_env/error, latent_row and the nested invariant row-order unit
test: fresh copies or one-sided simplification must not erase the shared
argument equation. The reference's invariant-occurrence guard is a useful
local rule already present in K4's traversal. Its richer interval/negation
representation is not a simpler replacement under i's equality rule. No
additional implementation change was made solely from the analogy.


The complete QA audit also found 118 baseline-accepted programs that now
reject: incompatible same-constructor services (including reordered groups,
unused invalid callbacks and nested A/B rows) or unprovided entry services.
The previously unclassified 68 cases now have explicit reject requirements
and run-before-entry checks. Their source families were inspected separately
from the observed diagnostics. Generic service conflict examples require
S(Int) and S(String) in one row; missing_* entries install S(Dep) without
providing Dep. The nested f2/f3/f4 examples require two incompatible A forms.
These follow §4.5 rule 4 and §9.2, including unused invalid definitions.
The new `run_stdout` requirements preserve the former explicit empty outputs
and r4's four/three output, rather than deriving semantic assertions from
fresh observations.

Sixteen K4 resource cases complete: fourteen at the explicit ten-second
capture budget, mutual300 and wide_generic_2048 at sixty seconds. The latter
commands finish all checker/startup/O0/O2 phases; their total command times
are about 75 seconds each, not isolated checker timings. An increased budget
is an observation change, not evidence of a speedup. The catalogue separately
measures the 100-function ring. The T1 stress-nested-50 known marker was
already obsolete: its saved redesign observation has all phases at exit 0.
Removing that marker is a metadata correction, not a K4 repair.

Per-case actual.json files are diagnostic scratch, now ignored and removed
from tracking in the derived-files commit. No expectation is removed: every
expected.json stays checked, censored raw observations stay in those files,
and the original QA revision retains the historical diagnostic scratch.


## QA integration results and remaining observations

All 119 K4 known-failure entries are promoted individually: 14 F1, 10 F2,
40 F3, one original annotated-entry regression, 38 other retained-service
entries, and 16 stress completions. The complete list, source sentence and
fixing finding for each is generated in tests/review/k4-integration.json/md.
There is no remaining K4 known-failure entry. Removing the already-passing
T1 nested-50 marker leaves three T3 held projects and two T1 runtime-test
stress observations; both of those have completed checker results.

The audit accounts for 976 changed observations against d655684b: 118
baseline-accepted programs now reject for incompatible/unprovided services;
other changes are finite inference, restored missing schemes, canonical
rows, named diagnostics, entry-run coverage and completion of censored phases.
Every new rejection has an explicit semantic requirement. All 14 changed
entries outside the K4 reviews are listed separately: JS4 settings only
reorders Await/Fs/Owner; nine Std-2 cases gain finite acceptance or a formerly
missing example scheme, two Std-2 negatives retain rejection with improved
service diagnostics; two T1 scalar-key cases complete previously censored
runtime phases. T1 error_rows is restored to its unchanged baseline.

The long QA generation command is scripts/test-generation --mode long
--seed 412 --count 180. It passes 720 model programs (504 accepted, 216
rejected by the coverage model), 1,440 declaration permutations and 720
annotation removals, with no failure. Its optional external model replay
was not requested or run. The stored long-run comparison is in the audit.

The stored QA benchmark catalogue is run under cache/quiet, five samples
per workload, with its ten-second observation budget. All 31 workloads
complete. The 100-function service ring now has median wall 0.393885 s,
CPU 0.37 s and peak RSS 17,588,224 bytes; the stored baseline times out at
10.167917 s and 4,065,640,448 bytes. No completed-cost ratio is assigned to
that censored baseline. Every wall/CPU/RSS difference and output hash is
retained in bench-k4.json, bench-k4-comparison.jsonl and the integration audit.

The stored snapshot comparison is not uniformly faster: wide-union-1024
is 0.865795 -> 1.105737 s wall (+27.7%), CPU 0.84 -> 1.07 s; std is
0.305605 -> 0.330411 s wall (+8.1%). These historical differences are
reported without attributing them all to noise or claiming a speedup.
A separate 30-pair interleaved std comparison against the actual redesign
compiler gives 0.328205 -> 0.324563 s wall, CPU 0.31 -> 0.31 s and RSS
55,252,992 -> 54,837,248 bytes: no std regression in that contemporary
control. The benchmark has no numeric gate. The wide-union cost remains
a measured performance follow-up; this integration makes no claim that
all checker workloads became faster.

Redesign d655684b has exactly the same crates, lib, std, tests/spec, spec
and Cargo input blobs as 82375c0. The isolated 82375c0 binary is therefore
the actual redesign compiler for the paired comparison, not a substitute
ring baseline (qa-baseline-equivalence.json records every tree/blob ID and
the binary hash). Std, hello and bench still have 2,197/5/23 bindings,
with exactly the same three std row-order permutations and no missing or
added bindings (types-qa-comparison.json).


## QA verification and incidents

The complete pre-final replay passes at source head 14fa7651:
H env UPDATE=1 cargo test -p ilang-check --test types --locked;
H env ILANG_BROWSER_PORT=4185 ./scripts/dev-verify;
H cargo test -p ilang-cli --test spec --locked;
H env WAT_DUMP=<lane scratch> cargo test -p ilang-cli --test wat_dump --locked;
and old-syntax-check outside that outer wrapper. H is the heavy/dev-shell
wrapper defined earlier. The gate includes all 1,935 corpus projects: 1,098
accepted, 837 rejected, five explicit XFAIL entries (three T3 held and two
T1 runtime observations), zero failed requirements and zero checker timeouts.
The short model run passes inside dev-verify, alongside all Rust/i/JS/browser
checks. All 242 spec cases pass at both optimization levels. The syntax sweep
parses 3,683 extracted snippets with exact diagnostics and 20 inherited
future-track entries.

All 906 redesign export records, including 577 actual WAT modules, remain
byte-identical. Only the same four generic_service_rows exports are added.
No existing checker golden changes in this follow-up. The corpus observations
are regenerated explicitly; all 1,855 old records have new compiler/binary
metadata and a ten-second budget (two K4 stress records use sixty seconds).
Those metadata changes are counted separately from the 976 changed outcomes.
Resource budgets are observation caps under docs/testing.md, not weakened
performance checks. Successfully promoted K4 cases now require completion;
none retains a known-failure exemption.

The integrity command verifies 1,880 preserved payload occurrences, 1,456
maintained mappings and 4,108 historical tools/evidence payloads with zero
failures. Long-generation model counts, verdicts, permutations, annotations
and report-only results are identical to the stored long run. Benchmark
output hashes differ only for the formerly censored service ring and std's
three canonical row permutations.

New failed checks/incidents in this follow-up:

- The initial full corpus run fails against pre-K4 observations and known
  markers, intentionally exposing XPASS and changed diagnostics/types. It
  also exposes the genuine T1 scalar-error subtraction regression, fixed
  in 3e3ccc6f. The temporary traced check reproduces that E0515; the fixed
  check and both runtime optimizations then pass.
- The first migration lookup compared formatted new bytes with unformatted
  preserved bytes and reused no projects. It was corrected to canonicalize
  temporary copies for matching; only its own newly generated duplicates
  were removed. No original review source was changed or lost.
- The initial three-second observations time out on several finite stress
  cases. Explicit ten-/sixty-second observations establish their completion;
  these are not claimed as isolated timing measurements. The two T1 runtime
  observations remain censored and visible.
- The prior-art document is absent from redesign despite the ledger's
  landed statement. The immutable 3beae77 note and all requested reference
  functions were read directly; none was edited or built.

No verification run was interrupted in this follow-up. No global config,
lockfile, shared checkout, reference implementation or other lane changed.
Per-command environment variables expired normally. Scratch logs and normal
browser dependencies remain in this lane. The final-head replay is recorded
separately in target/k4d-checks/qa-final-head-gates.log after the derived-files
commit; no source changes are planned after that commit.

Follow-ups outside this integration: the coordinator/Std-2 owner must promote
the five waiting logging probes in std/src/log.i, std/test/log_test.i and
docs/design/std-2.md after K4 lands; the coordinator should restore the missing
prior-art note to redesign. The stored wide-union performance increase is
reported as a measurement follow-up, with no inferred source attribution.
Only Solver::handle changes in compiler source in this QA follow-up. Other
new functions are the migration/audit helpers and corpus selection, runtime
observation and requirement checks; the earlier complete compiler function
inventory still applies to K4 as a whole.


The final cached diff check caught a generated trailing blank line in
k4-integration.md. The generator now writes exactly one final newline.
The unpublished derived-files commit was reset softly on this lane only,
preserving every worktree/index file, so that the generator correction and
this incident record precede the regenerated data commit. No source program,
semantic expectation or successful test result changed. The corrected
generated diff must pass git diff --cached --check before committing.
