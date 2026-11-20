# Reviewing the e4 comment merge

Baseline: `e975c5e935ff0237fa34a5f11a5a04ed70c0aa44`.
The merge preserves e4 code, test bodies, expected diagnostics and deletions.
Every scoped source file was first restored from that baseline, before matching
cleanup entries were replayed and stale contexts were rewritten. The resulting
exact map is `scripts/doc-comments-rewrites/final-tree.json`; see
[the application procedure](doc-comments-apply.md) for replay and checks.

All 30 conflicted paths were resolved as follows. “Kept” refers to e4's version;
only the prose cleanup was then applied.

| Conflicted path | Resolution |
| --- | --- |
| `crates/ilang-check/src/hir.rs` | Kept rest/spread/group fields and ordinary Fail representation; rewrote field explanations for the current service-key constraints. |
| `crates/ilang-check/src/providers.rs` | Kept deletion; did not revive the provider implementation. |
| `crates/ilang-check/src/resolve/native.rs` | Kept rest/provide resolution and all e4 branches; removed incidental rule citations. |
| `crates/ilang-check/src/types/display.rs` | Kept single-row and ordinary Fail rendering; cleaned citations in explanatory prose. |
| `crates/ilang-check/src/types/infer.rs` | Kept one need row, Fail handling, variance and handler witnesses; replaced obsolete stage-history header with current invariants. |
| `crates/ilang-check/src/types/mod.rs` | Kept e4 types and exports; replayed only matching comment rewrites. |
| `crates/ilang-check/tests/coalescing.rs` | Kept test programs and expectations; explained supplied-service construction without restoring obsolete key restrictions. |
| `crates/ilang-check/tests/generic_services.rs` | Kept constructor-cycle and rest-service fixtures; removed the obsolete claim that whole-row capture is unsupported. |
| `crates/ilang-check/tests/provider_graph.rs` | Kept deletion. |
| `crates/ilang-check/tests/row_schemes.rs` | Kept directed-row and Fail acceptance/rejection tests; replaced rule labels with the tested constraints. |
| `crates/ilang-check/tests/types.rs` | Kept test registration and fixture handling; cleaned incidental citations. |
| `crates/ilang-cli/tests/providers.rs` | Kept deletion; e4 replacement diagnostic tests remain intact. |
| `crates/ilang-cli/tests/wasi.rs` | Kept three-argument Continuation fixtures and all expected results; rewrote only comments. |
| `crates/ilang-syntax/examples/legacy/src/print.rs` | Kept e4 printer, including removal of is_match; no older code restored. |
| `crates/ilang-syntax/examples/legacy/src/syn3/convert.rs` | Kept rest/provide and ordinary Fail conversion; removed migration-stage prose. |
| `crates/ilang-syntax/examples/legacy/src/syn3/lower.rs` | Kept current adapter code; explained its isolated migration-fixture role without stage history. |
| `crates/ilang-syntax/src/facts/pre.rs` | Kept current facts and node layout; replaced historic progress inventory with stable-address and membership constraints. |
| `crates/ilang-syntax/tests/syn3_lower.rs` | Kept current test bodies and expected output; cleaned citations and old review labels. |
| `crates/ilang-wasm/src/emit/workers.rs` | Kept worker ABI and e4 entry stubs; no provider plans restored. |
| `crates/ilang-wasm/src/lower.rs` | Kept e4 lowering and $adapter_pin_call; cleaned surrounding constraints only. |
| `crates/ilang-wasm/tests/control_lazy.rs` | Kept Continuation fixtures, compiled programs and expectations exactly; cleaned comments. |
| `stdlib/src/assertion.i` | Kept current assertions and Fail calls; removed obsolete duplicate signature documentation. |
| `stdlib/src/fail.i` | Kept ordinary Fail implementation and imports; removed comment citations. |
| `lib/i/failure.i` | Kept deletion; no obsolete failure alias restored. |
| `std/src/calendar.i` | Kept Fail-based API and evaluation order; removed duplicate signature inventory and comment citations. |
| `std/src/cli.i` | Kept Fail-based API and evaluation-order temporaries; removed duplicate signature inventory and comment citations. |
| `std/src/float.i` | Kept e4 numerical implementation and evaluation order; cleaned documentation citations. |
| `std/src/list.i` | Kept e4 list implementation and Fail behavior; cleaned documentation citations. |
| `std/src/time.i` | Kept e4 time implementation and Fail behavior; cleaned documentation citations. |
| `std/test/uri_test.i` | Kept e4 URI fixture code and expectations; removed incidental test-comment references. |

The source-token proof checks every existing tracked Rust file and the library,
standard-library and three extra checker-fixture sources. Exact literals,
fixture directives and all seven Rustdoc example blocks are compared separately.
No compiler-source executable tokens or expected diagnostic strings differ from
e4. The one added Rust source is the cleanup enforcing test, which runs the
policy checker and its Python regression suite.

No language-rule references or TODOs were retained in audited comments. The
seven Rustdoc blocks are unchanged; none is an executable Rust doctest. Two
format-name mentions inside the migrator usage example remain protected.

The final nonheavy checks produced:

```text
comment policy: 0 unapproved reference or stale-comment lines
proof scope: new enforcing test crates/ilang-cli/tests/doc_comments.rs
comment-only proof: 439 source files, 245 changed, 0 differ beyond comments
rustdoc proof: 7 fenced/indented example blocks, no changes
recorded 1640 contextual rewrites in 245 files
replayed exactly 1640, refused 0
Python cleanup regression suite: Ran 9 tests; OK
Snippet checker regression suite: Ran 10 tests; OK
```

The decision to preserve all executable text follows the coordinator's
comment-only scope and the spec's comments paragraph: “Comments have no effect
on the meaning of a program.” The cleanup introduces no language decision.

Release validation against merge commit `25b8a3363` succeeded:

```text
H direnv exec /home/agent/Programming/i cargo build --locked --release -p ilang-cli --bin i
    Finished `release` profile [optimized] target(s) in 2m 05s
```

Here and in the command sequence, `H` denotes
`/home/agent/Programming/i/.rd/cache/heavy`. Unit/binary tests select all ten workspace crates. A separate
`-p ilang-syntax-migration --lib` invocation covers the excluded helper
library, whose sources are also part of the comment cleanup.

The initial expanded unit command included
`-p ilang-syntax-migration --lib --bins` as well as all ten workspace crates.
It exited 101 while compiling the excluded `migrate-syntax3-pipe` test binary:

```text
error[E0063]: missing fields `group` and `spread` in initializer of `ilang_syntax::ast::Stmt`
error[E0063]: missing field `rest` in initializer of `ilang_syntax::ast::Param`
error: could not compile `ilang-syntax-migration` (bin "migrate-syntax3-pipe" test) due to 2 previous errors
warning: build failed, waiting for other jobs to finish...
```

Both constructors are in `crates/ilang-syntax/examples/legacy/support/legacy/parser.rs`; their
executable tokens are unchanged from e4, as the source-token proof confirms.
The coordinator's code-retention instruction prevents repairing these AST
initializers in the comment lane. The adjusted checking tests all ten
workspace packages with `--lib --bins` and separately tests the migration
helper's library with `--lib`; the excluded helper binary is a reported
baseline limitation, not a changed expectation or weakened test.

The final review also removed bare numbered decision labels and two descriptions
of a former migration stage. The policy matcher now rejects those forms, with a
regression test that keeps legitimate runtime tracing and decision-tree prose
allowed. Its expanded detector counts 584 stale-comment lines at e4 and zero in
the cleaned tree. The exact replay map contains 1,640 entries in 245 files.

Final validation completed on 2026-11-15 (HKT). Compiler/library
source is frozen at `885b8a2f3`; the subsequent report commit changes this
checking documentation only. All commands below ran sequentially through
`H=/home/agent/Programming/i/.rd/cache/heavy` where shown. Test commands use
`RUST_TEST_THREADS=2`. Outputs show each command's last nonempty line; an empty
log is recorded as successful with no output.

| Checking command | Result and last line |
| --- | --- |
| `git submodule status crates/ilang-binaryen/binaryen` | `fba5005a132d85d578be5bd010e0487ed8e2ce3e (version_133)` |
| `"$H" direnv exec /home/agent/Programming/i cargo fmt --all` | Exit 0; `✓ Running devenv:enterShell in 68.2ms` |
| `scripts/doc-comments scan --base e975c5e93` | JSON inventory: counts in the application procedure; last line `}` |
| `scripts/doc-comments scan` | JSON inventory: zero reference/stale/TODO counts; last line `}` |
| `scripts/doc-comments record --base e975c5e93 --output scripts/doc-comments-rewrites/final-tree.json` | `recorded 1640 contextual rewrites in 245 files` |
| `scripts/doc-comments apply --all --check-replay e975c5e93` | `replayed exactly 1640, refused 0` |
| `python3 -I scripts/test-doc-comments.py` | 9 tests; `OK` |
| `scripts/doc-comments check --proof e975c5e93` | 439 existing sources, 245 changed, 0 beyond comments; `rustdoc proof: 7 fenced/indented example blocks, no changes` |
| `"$H" direnv exec /home/agent/Programming/i cargo build --locked --release -p ilang-cli --bin i` | Initial: ``    Finished `release` profile [optimized] target(s) in 2m 05s``; refreshed after final comments: ``    Finished `release` profile [optimized] target(s) in 53.93s`` |
| `"$H" direnv exec /home/agent/Programming/i env RUST_TEST_THREADS=2 cargo test --locked -p ilang-binaryen -p ilang-check -p ilang-cli -p ilang-host -p ilang-lsp -p ilang-macro -p ilang-package -p ilang-runner -p ilang-syntax -p ilang-wasm --lib --bins` | 288 passed, 4 ignored; `test result: ok. 28 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.44s` |
| `"$H" direnv exec /home/agent/Programming/i env RUST_TEST_THREADS=2 cargo test --locked -p ilang-syntax-migration --lib` | 24 passed, 0 ignored; `test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.20s` |
| `"$H" direnv exec /home/agent/Programming/i env RUST_TEST_THREADS=2 cargo test -p ilang-syntax --test format --test lisp_syntax --test lisp_syntax3 --test lisp_definition_rows --test syn3 --test syn3_lower --locked` | 147 passed, 0 ignored; `test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s` |
| `"$H" direnv exec /home/agent/Programming/i env RUST_TEST_THREADS=2 cargo test -p ilang-check --test library --test query --test coalescing --test row_schemes --test generic_services --test rest_services --locked` | 113 passed, 0 ignored; `test result: ok. 50 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 313.71s` |
| `"$H" direnv exec /home/agent/Programming/i env RUST_TEST_THREADS=2 cargo test -p ilang-cli --test doc_comments --test check --test provider_diagnostics --test rest_services --test wasi --locked` | 34 passed, 0 ignored; `test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 57.91s` |
| `"$H" direnv exec /home/agent/Programming/i cargo fmt --all --check` | Exit 0; `✓ Running devenv:enterShell in 18.4ms` |
| `"$H" direnv exec /home/agent/Programming/i cargo clippy --workspace --all-targets --locked -- -D warnings` | Exit 0; ``    Finished `dev` profile [unoptimized + debuginfo] target(s) in 13.86s`` |
| `direnv exec /home/agent/Programming/i scripts/check-snippets` | Exit 0; `old-syntax-check: 5215 extracted literals/blocks parsed; exact expected diagnostics match; 4 future-track entries` |
| `"$H" ./target/release/i fmt --check lib std bench examples` | Passed before and after release refresh; `no output; exit 0` |
| `"$H" ./target/release/i check --project std` | Passed before and after release refresh; `no output; exit 0` |
| `scripts/doc-comments check --proof e975c5e93 --i-bin target/release/i` | Passed before and after release refresh; `i fmt --to is proof: 45 files, 0 differ` |
| `python3 -I scripts/check-snippets-test` | 10 tests; `OK` |
| `git diff --check` | No output; exit 0 |

The initial expanded unit command differed from the final workspace unit command
by adding `-p ilang-syntax-migration` alongside `--lib --bins`. It exited 101;
its last line was `warning: build failed, waiting for other jobs to finish...`.
The two E0063 errors and the reason for preserving that e4 code are recorded
above. No test or expectation was deleted, weakened or re-recorded.

All 606 selected Rust tests passed; four pre-existing ignored literal-performance
tests remain ignored. Their names are `t1_stress_alias_1000`,
`t1_stress_calls_5000`, `t1_stress_match_2000` and `t1_stress_nested_50_by_50`.
All nine cleanup-tool and ten snippet-checker Python tests passed.

The release refresh changed the binary hash, so all three release-binary checks
were repeated on the refreshed artifact rather than relying on the first
build. Its SHA-256 is
`667488017095d76eab94e27aeb4444b5bea346792579ea866374591ee6fa2444`.
The final proof output is:

```text
comment policy: 0 unapproved reference or stale-comment lines
proof scope: new enforcing test crates/ilang-cli/tests/doc_comments.rs
comment-only proof: 439 source files, 245 changed, 0 differ beyond comments
rustdoc proof: 7 fenced/indented example blocks, no changes
i fmt --to is proof: 45 files, 0 differ
```

There are no retained audited language-rule references or TODOs. The only new
Rust source relative to e4 is the cleanup enforcing test; the proof reports it
separately from the 439 existing sources. The seven byte-identical Rustdoc
example blocks have text/sh fences, so no executable Rust doctest changed.
The two protected format-name mentions belong to the migrator's active usage
example, whose exact text is preserved.

The excluded legacy helper's binary tests could not run due to the unchanged e4
AST compile errors. The four ignored performance tests were not run. Full
workspace tests, the unfiltered conformance suite, and the review corpus remain
the coordinator's combined gate with fmtgap. Standard-library O0/O2 tests and npm
documentation example/build checks were not repeated after this e4 merge; their
earlier cleanup run is recorded in the prior lane report. No corpus record or
expected diagnostic changes relative to e4 are expected or included.
