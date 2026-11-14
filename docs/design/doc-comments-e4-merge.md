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
| `crates/ilang-check/src/resolve/three.rs` | Kept rest/provide resolution and all e4 branches; removed incidental rule citations. |
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
| `lib/i/assertion.i` | Kept current assertions and Fail calls; removed obsolete duplicate signature documentation. |
| `lib/i/fail.i` | Kept ordinary Fail implementation and imports; removed comment citations. |
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

Both constructors are in `examples/legacy/support/legacy/parser.rs`; their
executable tokens are unchanged from e4, as the source-token proof confirms.
The coordinator's code-retention instruction prevents repairing these AST
initializers in the comment lane. The adjusted verification tests all ten
workspace packages with `--lib --bins` and separately tests the migration
helper's library with `--lib`; the excluded helper binary is a reported
baseline limitation, not a changed expectation or weakened test.

The final review also removed bare numbered decision labels and two descriptions
of a former migration stage. The policy matcher now rejects those forms, with a
regression test that keeps legitimate runtime tracing and decision-tree prose
allowed. Its expanded detector counts 584 stale-comment lines at e4 and zero in
the cleaned tree. The exact replay map contains 1,640 entries in 245 files.
