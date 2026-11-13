Branch: `lane/fa`. Last committed, landable hash: `baf2e3b11b46e604a42a6eb82f6885540565898a`.
That checkpoint has the full requested green matrix through conv `31ecae801`.
The worktree is a pending resolved merge of faD `549e505102ff84f96d81dd8a1a5eef2e0355c7a8`
and latest conv `a5992fda16cbc7a7f200bf6058adc7d0380f4e43`. No red commit was made.

Steps 2c and 2d are committed green. Latest conversion integration and the
step-3 switch remain uncommitted until their complete final-source gate is
green. Step 4 is prepared only; `syn3/lower.rs` and the old tree have not
been deleted. Applying deletion before step 3 is green would violate the
coordinator's order.

The native resolver now covers the later shape groups, keyed selection
tables/calls, parameter data, written branch result unions, Fail error
channels, quote captures and source-fragment facts. The old ten extra
modules and extra program were missing later forms; they are implemented
and remain in the strict HIR/diagnostic differential. A genuine discrepancy
labelled synthetic `%arg` parameters; matching the existing unlabelled rule
fixes it. Query uses a native span/name index and retains main's late
binder/reference handling; LSP, literals and expand use native syntax.
The differential was never normalized or weakened.

Latest conv fixes the closed-function provider generalization regression.
Its native port keeps closed function-valued locals as bindings, while
open contracts and scalar providers still rejoin. The planner retains
lexical generic context, standalone type metadata, and written/generated
Lisp match behavior. Original `types/soundness.i` and `soundness.out` are
unchanged; the exact four-E0515 regression passes. The temporary fixture
grouping used during diagnosis was removed. The tentative early solver
experiment was also removed completely; no solver change remains.

The decisions rest on these controlling paragraphs:

- `syn3-final-ast.md` step 3: “`resolve.rs` reads the new tree; `lower.rs`'s
  rows of §1 become resolver code.” Native resolution and strict oracle
  comparisons preserve the transition semantics.
- Spec §4.7 Generalization: “A binding is generalized when its value is a
  direct value (§6.10): the type variables of its type that are not in the
  types of names from outside it become general, so each use of the name
  may give them different types.” This requires preserving closed provider
  schemes and their deferred Reader obligations.
- Syntax-three §7.3: “Environment statement `! T = e` followed by the rest
  `r` of a block” becomes “`{ r } <- (e)`”; spec §9.2: “A provided value of
  type `P` satisfies the need entry `S` when `P <: S` (§4.5 rule 4).” The
  converted written provider contract must survive; both p1 copies again
  reject with exactly one E0515.
- Syntax-three §2.3: “A data type has no spread.” The one formatter-skip
  addition records the deliberately restored invalid review-rec source;
  every formatting assertion remains, with an explanatory comment.
- Step 4 explicitly preserves a private migration tree under examples and
  raises syn3 to the crate root. Adapted fa2 scripts and an application
  guide are in `target/fa-night/deletion-application.md`. They also retain
  every old-node oracle assertion and the public parser's validation errors.

`schema.null_round` passes unchanged on the committed checkpoint at O0/O2
and on native step 3 at O0 before the last conv fix. The earlier fa2 failure
was not reproduced. Its original cause is unverified; no speculative
backend fix or altered expectation is claimed.

At the 02:00 merge deadline, the candidate is frozen and remains uncommitted.
No step-three or deletion commit is claimed green. The already-running std
O2 process is allowed to finish; its scheduler is paused so no subsequent
heavy check starts after the deadline.

| Evidence | Result |
|---|---|
| Last committed checkpoint | Full requested matrix green at `baf2e3b11`; steps 2c/2d complete |
| Post-a599 workspace check and workspace clippy `-D warnings` | Pass after the provider port and lint repairs |
| Post-a599 syntax/macro, with legacy feature | 272 pass |
| Post-a599 affected checker: lib, rows, types, resolver differential | 103 pass, four existing ignored |
| Final-source seven CLI binaries | 144 pass |
| Final-source std O2 | 2170/2170 pass at 02:01:53, after the merge deadline; wrapper kernel exit status 0 verified |
| Earlier native snapshot | Full Rust 690 collectively after focused repairs; examples 13 + existing skip; std O0 2170; spec 6/80, 7/104, 9/139 at both levels |

A small standalone-expression metadata-forwarding addition was made while
the post-a599 oracle binary was running. It is compiled and exercised by
the final CLI run; the complete final-source oracle/static matrix still
needs repetition. Do not treat the earlier snapshot's results as an exact
final-source full gate.

The first non-green step is latest-conversion validation plus step 3.
There is no known remaining failing test in the latest attempted targets;
the blocking set is uncompleted verification: full ilang-check integration
(skip only `spec_corpus_reverse_order_and_recovery`), final LSP, macro
examples, std O0, chapters 6/, 7/, 9/, 15.7/, and final workspace/source fmt
and clippy repetition after the last forwarding change. The already-running std O2 command completed after the deadline; no further
heavy checks started and the paused scheduler was then stopped by its own PID. Step 4 is gated on this set
and is therefore unapplied; its entire deletion gate is unverified.

Compiler source is frozen before the deadline. The merge is deliberately
preserved, fully staged for review, with no conflict entries. No reset,
stash, push, main-checkout branch change, diagnostic re-recording or test
weakening was used. All heavy commands were serial and wrapped. Early
plain `rustfmt` was unavailable and was retried through direnv; standalone
formatting/parser checks were light operations. `git diff --cached --check`
passes at the freeze.


The final report is written after the outstanding read-only std command
completed. No compiler source was changed and no commit was made after the
deadline. No active or queued heavy command from this lane remains.
The pending merge parents are still faD549 and conv a599; all compiler
changes and this report are staged, with no conflict entries.

# fa verification ledger

All heavy commands ran serially through `/home/agent/Programming/i/.rd/cache/heavy`
(`H` below), in `/home/agent/Programming/i/.rd/fa`, using the required main
`direnv` environment. No unfiltered spec/corpus or workspace test run was used.
The original command and full output of each row remain under `target/fa-night/`
as `<label>.command` and `<label>.log`; scheduler exits are in `gate-results.txt`.

These are chronological snapshots, including failed attempts. A passing
historical snapshot is not a claim about the final source. The final gate
is the `step3` rows, plus the passing focused repairs of its only two failed
tests. `step3-rust` itself retains exit 101; the repaired golden and formatter
were rerun without changing expected diagnostics or weakening assertions.

| Label | Exact command | Result |
|---|---|---|
| `conv-clean-check` | `$H direnv exec /home/agent/Programming/i cargo check --workspace --all-targets --locked` | 00:21:52: exit 0 |
| `conv-clean-clippy` | `$H direnv exec /home/agent/Programming/i cargo clippy --workspace --all-targets --locked -- -D warnings` | 00:21:40: exit 101 |
| `conv-clean-examples` | `$H ./target/debug/i test --project examples/macro-tests --opt 0` | 00:22:04: exit 0 |
| `conv-clean-fmt` | `$H direnv exec /home/agent/Programming/i cargo fmt --all -- --check` | 00:21:24: exit 0 |
| `conv-clean-source-fmt` | `$H ./target/debug/i fmt --check lib std bench examples` | 00:21:24: exit 0 |
| `conv-cli` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test macro_run --test typed_macros --test fmt --test lisp --test check --test commands --test macro_budget --locked --no-fail-fast -- --test-threads=2` | Failed; see historical explanations below |
| `conv-final-cli` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test macro_run --test typed_macros --test fmt --test lisp --test check --test commands --test macro_budget --locked --no-fail-fast -- --test-threads=2` | 22:28:53: exit 0 |
| `conv-final-examples` | `$H ./target/debug/i test --project examples/macro-tests --opt 0` | 22:29:04: exit 0 |
| `conv-final-rust` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-syntax -p ilang-macro -p ilang-check -p ilang-lsp --locked --no-fail-fast -- --skip spec_corpus_reverse_order_and_recovery --test-threads=2` | 22:23:18: exit 0 |
| `conv-final-std0` | `$H ./target/debug/i test --project std --opt 0` | 22:36:10: exit 0 |
| `conv-final-std2` | `$H ./target/debug/i test --project std --opt 2` | Cancelled while queued; test did not start |
| `conv-latest-check` | `$H direnv exec /home/agent/Programming/i cargo check --workspace --all-targets --locked` | 23:24:40: exit 0 |
| `conv-latest-cli` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test macro_run --test typed_macros --test fmt --test lisp --test check --test commands --test macro_budget --locked --no-fail-fast -- --test-threads=2` | Passing test output; final scheduler exit not separately recorded |
| `conv-latest-legacy-provider` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-syntax --features legacy --test syn3_lower --locked -- migrated_captured_row_provider_keeps_its_written_contract` | 23:24:46: exit 0 |
| `conv-latest-rust` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-syntax -p ilang-macro -p ilang-check -p ilang-lsp --locked --no-fail-fast -- --skip spec_corpus_reverse_order_and_recovery --test-threads=2` | 23:37:00: exit 101 |
| `conv-repaired-check` | `$H direnv exec /home/agent/Programming/i cargo check --workspace --all-targets --locked` | 23:42:53: exit 0 |
| `conv-repaired-cli` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test macro_run --test typed_macros --test fmt --test lisp --test check --test commands --test macro_budget --locked --no-fail-fast -- --test-threads=2` | 23:50:00: exit 0 |
| `conv-repaired-clippy` | `$H direnv exec /home/agent/Programming/i cargo clippy --workspace --all-targets --locked -- -D warnings` | 00:12:13: exit 101 |
| `conv-repaired-examples` | `$H ./target/debug/i test --project examples/macro-tests --opt 0` | 23:50:11: exit 0 |
| `conv-repaired-fmt` | `$H direnv exec /home/agent/Programming/i cargo fmt --all -- --check` | 00:12:09: exit 1 |
| `conv-repaired-rust` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-macro -p ilang-lsp --locked --no-fail-fast -- --skip spec_corpus_reverse_order_and_recovery --test-threads=2` | 23:45:01: exit 0 |
| `conv-repaired-source-fmt` | `$H ./target/debug/i fmt --check lib std bench examples` | 00:12:09: exit 1 |
| `conv-repaired-spec15.7` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test spec --locked -- 15.7/` | 00:12:06: exit 0 |
| `conv-repaired-spec6` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test spec --locked -- 6/` | 23:57:30: exit 0 |
| `conv-repaired-spec7` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test spec --locked -- 7/` | 23:59:57: exit 0 |
| `conv-repaired-spec9` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test spec --locked -- 9/` | 00:11:43: exit 0 |
| `conv-repaired-std0` | `$H ./target/debug/i test --project std --opt 0` | 23:56:26: exit 0 |
| `conv-repaired-std2` | `$H ./target/debug/i test --project std --opt 2` | 00:20:09: exit 0 |
| `conv-rust` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-syntax -p ilang-macro -p ilang-check -p ilang-lsp --locked --no-fail-fast -- --skip spec_corpus_reverse_order_and_recovery --test-threads=2` | 21:58:04: exit 0 |
| `step3-a599-check-affected` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-check --lib --test row_schemes --test types --test resolve3_differential --locked --no-fail-fast -- --test-threads=2` | 01:44:32: exit 0 |
| `step3-a599-check` | `$H direnv exec /home/agent/Programming/i cargo check --workspace --all-targets --locked` | 01:36:49: exit 0 |
| `step3-a599-cli` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test macro_run --test typed_macros --test fmt --test lisp --test check --test commands --test macro_budget --locked --no-fail-fast -- --test-threads=2` | 01:52:44: exit 0 |
| `step3-a599-clippy` | `$H direnv exec /home/agent/Programming/i cargo clippy --workspace --all-targets --locked -- -D warnings` | 01:37:40: exit 101 |
| `step3-a599-clippy2` | `$H direnv exec /home/agent/Programming/i cargo clippy --workspace --all-targets --locked -- -D warnings` | 01:38:24: exit 101 |
| `step3-a599-clippy3` | `$H direnv exec /home/agent/Programming/i cargo clippy --workspace --all-targets --locked -- -D warnings` | 01:39:12: exit 0 |
| `step3-a599-std2` | `$H ./target/debug/i test --project std --opt 2` | 02:02:38: exit 0 |
| `step3-a599-syntax-macro` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-syntax -p ilang-macro --features ilang-syntax/legacy --locked --no-fail-fast -- --test-threads=2` | 01:40:42: exit 0 |
| `step3-check` | `$H direnv exec /home/agent/Programming/i cargo check --workspace --all-targets --locked` | 00:36:23: exit 0 |
| `step3-cli` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test macro_run --test typed_macros --test fmt --test lisp --test check --test commands --test macro_budget --locked --no-fail-fast -- --test-threads=2` | 01:11:59: exit 0 |
| `step3-examples` | `$H ./target/debug/i test --project examples/macro-tests --opt 0` | 01:12:10: exit 0 |
| `step3-focused-differential` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-check --test resolve3_differential --locked -- --nocapture` | 00:36:06: exit 0 |
| `step3-focused-rows` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-check --test row_schemes --locked -- --test-threads=2` | 00:33:53: exit 0 |
| `step3-focused-walker` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-macro --test body_walk3 --locked -- --nocapture` | 00:32:28: exit 0 |
| `step3-legacy-provider` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-syntax --features legacy --test syn3_lower --locked -- migrated_captured_row_provider_keeps_its_written_contract` | 00:36:29: exit 0 |
| `step3-repair-differential` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-check --test resolve3_differential --locked` | 01:06:33: exit 0 |
| `step3-repair-format` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-syntax --test format --locked` | 01:04:43: exit 0 |
| `step3-repair-types` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-check --test types --locked -- --exact types` | 01:01:46: exit 101 |
| `step3-repair-walker` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-macro --test body_walk3 --locked` | 01:04:47: exit 0 |
| `step3-repair2-types` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-check --test types --locked -- --exact types` | 01:03:44: exit 0 |
| `step3-rust` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-syntax -p ilang-macro -p ilang-check -p ilang-lsp --locked --no-fail-fast -- --skip spec_corpus_reverse_order_and_recovery --test-threads=2` | 01:06:33: exit 101 |
| `step3-spec6` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test spec --locked -- 6/` | 01:20:08: exit 0 |
| `step3-spec7` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test spec --locked -- 7/` | 01:23:54: exit 0 |
| `step3-spec9` | `$H direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test spec --locked -- 9/` | 139/139 at O0/O2 on superseded binary; final scheduler exit not recorded |
| `step3-std0` | `$H ./target/debug/i test --project std --opt 0` | 01:18:58: exit 0 |

Historical findings and repairs:

- Initial conversion check: generated service-Fail alias lacked `alias_pattern`;
  set false for the ordinary named-type RHS, then the same check passed.
- Initial CLI captured-row E0505 was a missing written provider contract in
  conversion. A tentative solver experiment was rejected and removed. Final
  solver source has no such change. Five diagnostic frontend probes and their
  outcomes are described in `target/fa-night/verification.txt`.
- Native preview check: stale sum enum and unused call binding fixed; query
  label-binder mismatch fixed by retaining main's late pun/reference handling.
  Strict extra resolver comparisons exposed missing later forms and labelled
  synthetic parameters; all original comparisons were retained.
- Native walker whole-repository comparison: direct Fail(E) is an error channel,
  not a local binder. Fixed plain/wrapped scopes; ordinary aliases retain binders.
- Conversion formatting: eight Rust files and one macro-example layout; clippy
  collapse-if and needless Rust borrow fixed without assertion/input changes.
  `conv-clean-clippy2.log` records the final passing clippy rerun.
- Step3 initial all-target check: provider-plan helper `block` collided with the
  visitor method; renamed `flatten_block`. `step3-check2.log` passes the same
  wrapped workspace/all-target check before the final gate check passes again.
- Step3 full Rust run: 688 pass, two fail, four existing ignored, one requested
  recovery-corpus filter. Restoring an inferred callback did not recover the
  soundness diagnostics; that attempt was rejected. Grouping the provider use
  keeps the ordinary local generalization under test (§2.2), and the unchanged
  golden passes. The only formatter difference is the skip inventory for the
  restored intentionally invalid type spread (§2.3); all properties pass again.
- Standalone resolver-deletion preview files parse with `direnv exec
  /home/agent/Programming/i rustfmt --edition 2024 ... --config skip_children=true`;
  an initial plain `rustfmt` attempt found no executable. These are preparation
  checks only, not compilation or behavioral proof for deletion.
- `git diff --check` passed after the source and inventory repairs.

The complete running notes, diagnostic-probe details, queued-command
cancellation, and restoration checks are retained in
`target/fa-night/verification.txt`. No active test from another lane was stopped.


## Nov14 expanded step-three gate (2026-11-14T06:23:53.425280+08:00)

The native resolver source at `3499484118bc3bca6b127547c9cdbecf7c72f801` completes the requested expanded gate.
The merge/checkpoint commits preserve faD549, main68b720e9 and convdb743.
Production resolution, queries, literals, editor analysis and expand read the
native tree. The remaining old resolver/lowerer are compatibility oracles
until step four. Native resolver fixes do not edit diagnostic goldens;
upstream conversion snapshots retain their merge provenance.

The final four-package Rust run reports 752 passed, 0 failed
and 4 existing ignored tests; only the requested recovery test is
filtered. All 39 CLI integration binaries plus the CLI library pass (one
existing ignored benchmark). Both std optimization levels pass 2170/2170,
including the unchanged schema.null_round assertion; macro examples pass13
with one existing skip. Spec6 passes80, spec7 passes104, spec9 passes139 and
spec15.7 passes28, at both optimization levels; existing pending entries
are retained. The short generation model, Rust/source formatting and strict
workspace clippy pass. The final Rust repeat includes all restored comment
APIs and their tests.

Every command below uses the shared wrapper, and commands ran serially.
The initial resumed workspace check found a misplaced import before inner
module docs (E0753); moving it after the docs fixed it. The first expanded
clippy found one needless name borrow; the final-source strict rerun passes.
Neither repair changes language behavior. Original main comment conversion
APIs, two original deep tests and the old decoder negative are retained.

- `resume-main-check`: exit 101.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo check --workspace --all-targets --locked
```

- `resume-main-check2`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo check --workspace --all-targets --locked
```

- `resume-alias-focused`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo test -p ilang-check --test row_schemes --test resolve3_differential --locked --no-fail-fast -- --test-threads=2
```

- `step3-check`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo check --workspace --all-targets --locked
```

- `step3-fmt`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo fmt --all -- --check
```

- `step3-clippy`: exit 101.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo clippy --workspace --all-targets --locked -- -D warnings
```

- `step3-rust`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo test -p ilang-syntax -p ilang-macro -p ilang-check -p ilang-lsp --features ilang-syntax/legacy --locked --no-fail-fast -- --skip spec_corpus_reverse_order_and_recovery --test-threads=2
```

- `restore-check`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo check --workspace --all-targets --locked
```

- `restore-syntax`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo test -p ilang-syntax --features legacy --locked --no-fail-fast -- --test-threads=2
```

- `restore-fmt`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo fmt --all -- --check
```

- `restore-clippy`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo clippy --workspace --all-targets --locked -- -D warnings
```

- `step3-cli`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo test -p ilang-cli --lib --test big_int --test big_int_performance --test check --test cli_log --test collections --test commands --test compile_cache --test compile_time_tests --test determinism --test echo --test entries --test expansion_sites --test float_text --test fmt --test http --test lisp --test lsp --test macro_budget --test macro_run --test many_tests --test multicore --test native --test numeric_lowering --test package_commands --test providers --test r2_fixtures --test r2_performance --test r2_runtime --test spec_update --test syn3_fixtures --test syntax_runtime --test tls --test typed_macros --test udp --test unused --test value_type_performance --test value_types --test wasi --test wat_dump --locked --no-fail-fast -- --test-threads=2
```

- `step3-release`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo build --release -p ilang-cli --bin i --locked
```

- `step3-examples`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy ./target/release/i test --project examples/macro-tests --opt 0
```

- `step3-std0`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy ./target/release/i test --project std --opt 0
```

- `step3-std2`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy ./target/release/i test --project std --opt 2
```

- `step3-spec6`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test spec --locked -- 6/
```

- `step3-spec7`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test spec --locked -- 7/
```

- `step3-spec9`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test spec --locked -- 9/
```

- `step3-spec15.7`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test spec --locked -- 15.7/
```

- `step3-generation`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy scripts/test-generation --mode short --binary ./target/release/i
```

- `step3-source-fmt`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy ./target/release/i fmt --check lib std bench examples
```

- `step3-rust-final`: exit 0.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo test -p ilang-syntax -p ilang-macro -p ilang-check -p ilang-lsp --features ilang-syntax/legacy --locked --no-fail-fast -- --skip spec_corpus_reverse_order_and_recovery --test-threads=2
```

Additional preparation verification (all pass):

```sh
PYTHONPYCACHEPREFIX=target/fa-r3/pycache python3 -m py_compile scripts/syn3-delete-*.py scripts/syn3-capture-engine.py
python3 scripts/syn3-delete-old-tree.py
git diff --cached --check
```

The driver dry run changes no production file. Python syntax checks were
also repeated on the ignored preparation copies after each edit. A read-only
`git merge-tree --write-tree HEAD refs/heads/lane/s12` preview identifies one
append-only provider-test conflict; both sets of tests must be retained.
The short generation report has32 attempts,12 model rejections,64 permutations,
32 annotation removals,20 accepted cases and no failures. Permitted annotation
outcomes remain report-only as classified by the model.

The full workspace test run, unfiltered spec/corpus and dev-verify remain
coordinator gates and were not run. Deletion is still unapplied and step four
will be recorded separately.


## Step-four stage12 integration, 06:26

On `lane/fa-delete`, merging stage12 preserves all row-test functions from
both parents. No soundness golden is edited. The exact command below passes
(exit0; log target/fa-r3/s12-check.log). Deletion is still unapplied.

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo check --workspace --all-targets --locked
```


## Applied deletion audit (in progress, 07:08)

The original five-case engine test passes before deletion (1 passed,61
filtered), with both complete answers equal for every input. Capture uses:

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test macro_run --locked type_macro_keeps_its_standalone_need_channel -- --exact --nocapture --test-threads=2
python3 scripts/syn3-capture-engine.py collect
python3 scripts/syn3-delete-old-tree.py --apply
```

The exact clean head is5dd20acf9041859d74ad3c56ac22b21d232eecc2; the private
pre-stage12 oracle is green3=6de9a39b655edc8fad9dd7e14a848e359df22915. The
first two answers keep the macro-produced type's needN separate from the
written parameter's implicit errorN%E. The third keeps standalone%multi
groups; the fourth keeps a generated union as an alias; the fifth keeps
a written alias while an ignored macro argument becomes Int. Full strings,
all five original inputs and their equality assertions are retained.

Repeated all-target audit command (one wrapper at a time):

```sh
/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i cargo check --workspace --all-targets --offline --message-format=short
```

| Attempt | Result and diagnosed boundary |
|---|---|
|1|101: obsolete show_as branch and missing from_document factor; native readers wired. CLI runner build reports the same underlying errors.|
|2|101: private unused native renderer and missing shared quote/identifier/operator helpers; private duplication removed and HIR-only helpers retained.|
|3|101: syntax unit/dependency type identities and private resolver expansion calls; comparisons routed through shared normal dependency types.|
|4|101: internal normalizer helpers and remaining unit imports; production native normalization is shared with the old oracle.|
|5|101: one private item reader; replaced by equivalent public item_from_datum.|
|6|101: checker test helper still used the removed old tree; private compatibility imports retained.|
|7|101: optional warning-inventory old forms; native Macro/Bind/Chain reader keeps original counting conditions.|
|8|101: private binder visibility and backend benchmark spans; public private-oracle helper and native span access fixed.|
|9|101: R2 fixture generator old binding/attribute/header fields; native equivalents retain all transformations and assertions.|
|10|101: one native pattern name was still a String; direct comparison restored.|
|11|PASS: workspace all targets compile, production and test consumers included.|

Logs are target/fa-r3/delete-check1.log through delete-check9.log. No
diagnostic golden is edited to address a compile failure. Full deletion
tests, std, spec, generation, formatting and clippy remain pending.


07:16: attempt11 completes successfully. The sole warning belongs to the
non-workspace historical printer copy. No production normal dependency
points to ilang-syntax-migration (verified in all four consumer manifests).
`git diff --check` passes; no .out or .expect.json diagnostic golden changes.
This compiling checkpoint is not the full green deletion gate.
