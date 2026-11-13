# E4 integration with main stage twelve

Base: main `180e00cf70398034e50f253c0b209298758bb4d5`.
Merge: `72892af423bcac0bcb242ce2d23dd25b8a6f6325`.
The earlier JavaScript investigation and its green pre-merge gates remain in
[effects-e4-js.md](effects-e4-js.md); the operation table remains in
[effects-e4.md](effects-e4.md).

## Merge decisions

- `solve.rs::union_in` validates environment differences before eliminating
  vacuous or duplicate members. This retains main's named-function provider
  validation and E3's `Fail(Never)` need handling. Both provider diagnostic
  regressions remain, including their four conflict locations. No soundness
  golden loses a diagnostic. The actual `crates/ilang-check/tests/types/soundness.out`
  differs from main only in the `with_int` and `arity` type lines, which use
  the E3 whole row `N`; every diagnostic line is identical.
- Main's compiler legacy formatter deletion remains. The discarded lane change
  only ignored newly added structure fields in patterns; it had no formatter
  behavior to transfer to the test-support legacy reader.
- The generator keeps main's syntax-three source forms and literal-match helper.
  The service model retains its explicit `Svc(Int)` contract; generated error
  examples use `Fail` needs. Model expectations are unchanged.
- The two review-k4d b1/r6 observations are exactly main's bytes, as directed.
  The coordinator owns their final gate-tree recording.

The solver decision follows spec §9.2: “A provided value of type `P`
satisfies the need entry `S` when `P <: S`” and “An entry that every value of
its type constructor satisfies, as `Fail(Never)`, is removed.” The other
conflict choices follow the coordinator's explicit merge directions.

## Suite provider regression

`suite_items` previously recognized only a three-part environment statement.
Grouped provisions also carry group/spread metadata and can first bind temporary
service values. A provided nested expression can introduce a block. Retain the
complete environment nodes and bindings; traverse blocks with their local list
and resume siblings with the original outer list.

Spec §10.2: “`suite` prefixes names and provides the services written in it to
its test bodies; `each` contributes one test for each written case.”
Spec §9.2: “For a key that `s` has, `s` wins; the other keys come from outside.”

Six new collected cases exercise two provided services, a qualified nested
suite, two `each` cases, an inner override, restoration after the inner scope,
and an independent sibling. Every original assertion remains. The Rust macro
regression additionally requires the original service case and all six new
case names in the output, preventing a silently dropped test from passing.

The first focused expansion exposed temporary bindings being mistaken for test
calls; retaining those bindings fixed the explained failure. The subsequent
expansion and O0/O2 example runs pass (15 passed, 1 skipped each). All 36 tests
in `typed_macros` pass, including the reported regression.

## LSP hover expectations

The broad gate found two stale protocol expectations. Unknown callback types
were written as `(A) <- (Fail(E), ..N) -> B`; the E3 checker correctly gives
`(A) <- N -> B`, as already pinned by both matching checker-query tests.
Only those seven type strings are updated; all global/local naming assertions
and diagnostics remain. No compiler or language behavior changes.

syntax-3 §6.1: “**One need row.** `err` is removed (stage E3): errors are the
`Fail(E)` entry of `env`.” Spec §9.2 gives the whole-row example
`map : <A, B, N>(List(A), (A) <- N -> B) <- N -> List(B)`.
The first protocol run passes 96/98; both failures show exactly the explained
one-row difference. A complete corrected protocol run follows.

## Lazy control fixtures

The initial crate gate finds two obsolete E3 assumptions in `control_lazy`.
The trampoline dispatch test expects both `0x1C` and `0x0C`: the first mask
includes old catch kind 4. It now requires `0x0C`, rejects `0x1C`, and rejects
`$catch_reenter`, preserving explicit checks of the remaining record kinds.

syntax-3 §2.10.7: “**Removed built-ins.** `fail.raise` and `fail.catch` stop
being built-in: `Op::Raise`, `Op::Catch`, `$fail_catch`, `$catch_run`,
`$catch_reenter` and frame kind 4 are deleted (stage E3). The `$error` tag
stays, for `escape`.”

The pending-final fixture gives its paused continuation `Fail(Int)` but calls
`cancel` in a final function, which must have no Fail need. Under E3, cancel
passes on the continuation's whole need; the original no-error assumption
is obsolete. Capture the handler explicitly in `paused`, create that
continuation inside the still-active catch, and call `handler.raise(7)` in
the ordinary code after the inner boundary. The paused continuation then has
need `Never`, while its captured handler still escapes through the pending
final record. Both O0 and O2 copies retain the exact `Error(7)` output, empty
stderr and zero-exit assertions; no expected output changes.

Spec §9.4: `control.cancel` has type
`<A, R, N>(Continuation(A, R, N)) <- N -> Nil`.
Spec §9.2: “A service value can be captured. Providing the service later
changes calls that take it from their caller, not the captured value.”
Spec §9.3: “An error goes out through the calls to the `catch` that provided
its `Fail`.” The catch remains active throughout this fixture.

## Remaining integration test assumptions

The first complete CLI run also finds three determinism cases building
service-using examples for `host`, and one native output check doing the same.
The determinism cases now build `nodejs`, preserving their two-build byte
comparisons and expansion comparisons at O0/O2. The native test still checks
that `host` emits raw Wasm, using a service-free entry, and additionally checks
raw `nodejs` Wasm for hello. Both native main/named-entry output checks remain.
Spec §9.7's platform table supplies the needed IO/network services to nodejs;
syntax-3 §2.10.11 removes provider plans and makes target services explicit.

The R2 failure-order unit probe has two private worker stubs using the obsolete
four-parameter `Continuation`; only that type arity is corrected. All three
lexicographic/prefix ordering assertions remain at both optimization levels.
syntax-3 §2.10.7: “The error parameter leaves `Boundary` and `Continuation`.”

The macro frame regression's hard-coded 20-unit budget predates the ordinary
library catch/provider frames in E3/E5 entry setup. Measure the exact
zero-recursion entry peak, then require both 40 and 400 tail calls to fit that
same budget, with final frame usage zero. Forty non-tail calls must still
exhaust it. This pins flat recursion independently of the entry's constant
cost; it does not change resource accounting or the production limits. The
complete nine-test target passes. The measured baseline is 29 units, so the
old 20-unit limit already fails without recursion; both tail depths fit
the same exact 29 units and non-tail recursion still exhausts them.
`macro-budget.md` says “Tail calls leave the caller's frame before the callee
enters.” It also charges ordinary source functions and synthesized wrappers.

## Snippet registrations

An identity audit finds 77 stale records, all found in main's pre-E3/E5 source:
75 obsolete non-i classifications and two changed source-template fragments.
Drop the classifications for absent literals, rekey both current fragments
with the same render settings, and classify two reviewed root/dependency
TOML manifests written to `i.toml`. No diagnostic expectation is added,
removed, changed, or regenerated. The full parser gate runs afterwards.

## Verification on the merged tree

All heavy jobs use the shared wrapper, one job at a time. Cargo uses the main
checkout's direnv environment, with `CARGO_PROFILE_DEV_DEBUG=0` and
`CARGO_INCREMENTAL=0`. Rust test gates use the release profile. The custom spec
runner is run with each requested chapter filter, on two CPUs; it exercises
both i optimization levels. The checker skip is exactly
`spec_corpus_reverse_order_and_recovery`; no other test is skipped by request.

- Workspace all-targets check: pass after the merge and again on the final source.
- Requested six-crate tests: all pass after complete LSP protocol (98) and
  control_lazy (17) reruns; the other targets of the original run pass.
- Every CLI integration binary, lib/bin tests, and doc tests: pass after
  complete corrected target reruns (determinism 3, macro_budget 9, native 14,
  r2_runtime 8, WASI 17). `typed_macros` has its separate complete 36-test run.
- Std typecheck: pass.
- Std O0 and O2: each 2,171 passed, zero failures.
- Release CLI build: pass.
- Macro expansion and macro example tests O0/O2: pass.
- Complete CLI `typed_macros` target: 36 passed.
- `i fmt --check bench std lib examples`: pass.
- `cargo fmt --all -- --check`: pass.
- Spec `6/`: 80 passed at O0/O2; the existing 28 pending cases remain.
- Spec `9/`: 149 passed at O0/O2.
- Spec `9.7/`: 17 passed at O0/O2.
- Spec `9.9/`: 78 passed at O0/O2.
- Spec `11/`: 48 passed at O0/O2.
- Remaining requested gates: running; this record is updated before handover.

Commands (the following are the commands inside the wrapper):

```sh
cargo check --workspace --all-targets --locked
cargo build --release -p ilang-cli --locked
./target/release/i expand --project examples/macro-tests --trace
./target/release/i test --project examples/macro-tests --opt 0
./target/release/i test --project examples/macro-tests --opt 2
cargo test --release -p ilang-cli --test typed_macros --locked -- --test-threads 2
cargo test --release --no-fail-fast -p ilang-syntax -p ilang-check -p ilang-macro -p ilang-lsp -p ilang-wasm -p ilang-host --locked -- --test-threads 2 --skip spec_corpus_reverse_order_and_recovery
```

The other CLI binaries run together with `--lib --bins` and an explicit
`--test NAME` for every integration target except `typed_macros` (complete
run above) and `spec` (the five requested filtered runs). This covers every
CLI binary without an unfiltered spec run. Each uses `--test-threads 2`.
The exact remaining targets are:

```sh
cargo test --release --no-fail-fast -p ilang-cli --locked --lib --bins \
  --test big_int --test big_int_performance --test check --test cli_log \
  --test collections --test commands --test compile_cache --test compile_time_tests \
  --test determinism --test echo --test entries --test expansion_sites \
  --test float_text --test fmt --test http --test lisp --test lsp \
  --test macro_budget --test macro_run --test many_tests --test multicore \
  --test native --test numeric_lowering --test package_commands \
  --test r2_fixtures --test r2_performance --test r2_runtime --test rest_services \
  --test spec_update --test syn3_fixtures --test syntax_runtime --test tls \
  --test udp --test unused --test value_type_performance --test value_types \
  --test wasi --test wat_dump -- --test-threads 2
```
