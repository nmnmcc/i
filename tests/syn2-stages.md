# SYN2 stage record

Baseline: 2d1cfc56 on lane/syn2. The lane was clean. The study experiments
under target/syn2 were retained as evidence, not installed as production code.
The coordinator explicitly resumed the uncommitted stage-1 documents; they
were kept and the entry decision was folded into them.

## Sources and interpretations

The retained USER decisions control syntax. The entry decision removes @main
and selects the default from src/main.i.
The sentence “each may point at any function in any module of the package”
permits private functions in [entries]. Keys use ordinary name syntax; the
values reuse [wasm].exports paths. --entry chooses a named entry; metadata
keeps the selected function path so ABI v3 and both hosts need no new shape.
The sentence “main functions and [entries] of dependency packages have no
effect” restricts all discovery and declaration checking to the root package.
Duplicate binding names and TOML keys replace the old attribute-count error.

Ruling f explicitly removes empty tuples: migration flags them for manual review.
Ruling g requires ordinary shadowable Record lookup. No generated code in
this stage inserts Record; T3 must provide a qualified ordinary exported
alias before its record type printer claims capture-free source output. The hand-off name is
`i/record.All`: an ordinary exported alias defined where the built-in is visible.
Source generators import that module under a fresh local alias and use
`fresh.All`; diagnostics without an import use the canonical identity
`i/record.All`, as existing diagnostics use full module identities. This
requires T3 library integration; no keyword or privileged module lookup is added.

## Prior art

Read actual source before stage 1: Dart parser_impl.dart parseParenthesizedExpressionFunctionLiteralOrRecordLiteral
and parseParenthesizedExpressionOrRecordLiteral; Gleam parse.rs
parse_case_clause. Take matching-close lookahead and separate
head/guard/body parsing. Leave keywords and comma-based disambiguation.
Gleam format/src/lib.rs pretty and Intermediate::from_extra separate comment
positions from AST printing; retain i's existing equivalent structure.
Go cmd/gofmt/rewrite.go rewriteFile matches trees and preserves comments;
Cargo ops/cargo_fix/fix_edition.rs fix_edition verifies both source versions;
Cargo crates/rustfix/src/lib.rs CodeFix applies bounded text edits. Take
AST-directed edits and reparse checks, leave their language-specific rewrites.
Gleam compiler-cli/src/fix.rs run collects compiler evidence before edits;
its current fix is a manifest version repair, not a general syntax migrator.
Go cmd/fix/main.go delegates to analysis fixes; use separate migration rules.

For entries: Go cmd/go/internal/load/pkg.go mainPackagesOnly restricts entry
discovery to the selected main package; i uses the root package's src/main.i.
Gleam compiler-cli/src/run.rs setup selects a module and validates main;
i additionally allows arbitrary named functions through [entries].
Deno cli/tools/run/mod.rs resolves the selected main module before creating
the worker; i resolves an entry before lowering. Zig lib/std/start.zig
wasi_start/callMain wraps root.main; retain i's existing target wrappers.
Cargo workspace/parser/targets.rs is the source-convention/explicit-target
precedent; i has one default plus a small table, without target inference.

## T3 surface contract

The integration merge supplies ExprKind::Record { base, fields } and
TypeKind::Record { base, fields, removed }. Both surfaces construct these
nodes. The parser selects function suffixes first, preserves call and
constructor contexts, and does not add a comma to disambiguate.

## Verification

Checks and migrations will be recorded below as they finish.

Stage 1 measurements: the initial WAT_DUMP test passed and wrote 906 files.
A relative dump path was resolved below crates/ilang-cli/target; the output
was moved into this lane's target/syn2/wat-before and the two empty temporary
directories were removed. No tracked file outside the lane changed.
A separate baseline build-WAT sweep observed 3,777 tracked manifests, 2,685
WAT outputs and 15 watchdog timeouts. It includes historical manifest data;
it is a supplementary sweep, not proof that all snippets compile.
Checker baseline: scripts/checker-bench --filter std --runs 5, release
compiler at 2d1cfc56, wall median 0.291424 s (QA published median 0.306 s).
Raw data are in target/syn2/checker-before.json.

The first old-syntax-check passed (3,548 snippets, 29 future entries).
After the literal-types examples changed, the next run failed because one
content identity was stale. That future snippet was re-registered with the
exact baseline diagnostics; the check is rerun, without broadening policy.
The stage-1 full gate's Rust portion and O0/O2 standard/example/bench suites
passed; the full review corpus and JavaScript portion are still running at
this checkpoint. The source compiler used by that run was built before the
next stage; its release executable was saved as target/syn2/i-before.

Stage 1 final results: heavy direnv exec /home/agent/Programming/i env
ILANG_BROWSER_PORT=4188 ./scripts/dev-verify exited 0. Rust fmt, Clippy
-D warnings, workspace tests (including spec O0/O2), i tests O0/O2, WASI,
1,855 review projects under their existing policy, short generation, SDK
and all 51 browser tests passed. old-syntax-check retry exited 0 with
3,548 snippets and 30 exact future entries. git diff --check passed.
The review runner wrote one untracked actual.json for an allowed timeout
in stress-nested-50 test2; it was moved to target/syn2/stage1-stress-actual.json.
No policy, limit, lockfile, global environment or reference tree changed.
# SYN2 implementation notes

The pipe remains `|>` and a match remains a pipe into a clause block.
The new error marker is `^`; the default entry is `src/main.i`'s `main`,
with other entries declared in `[entries]`. Record nodes come from T3.

Integration note: old braced and new parenthesised record forms construct the
T3 record nodes. The parser uses `// syntax: syn2` or a `^` in a file to
choose guard interpretation where the old error marker is ambiguous with a
function-type guard; the migration rewrites error markers before that choice.
The Go `cmd/gofmt/rewrite.go` comment-map approach motivates retaining source
between span edits, and Cargo `src/workspace/parser/targets.rs` and Gleam
`compiler-cli/src/run.rs` distinguish conventional and explicitly selected
entry modules.

## Remaining verification issue

The full spec suite's second run passed 400/402 cases. The old record
reservation case now accepts a record and its expectation has been updated.
The remaining `tests/spec/10.2/main_and_test.i` reproduces an entry/test
collision with `@main\n@test\nboth = () { Nil }`: the single-file runner
constructs `src/main.i`, and reports E0305 (test placement) instead of the
expected E0321 (entry/test collision). Two attempts to reconcile this fixture
with the new single-file runner have not restored its expected diagnostic.

The `i/record.All` capture-free source form described above is not installed:
`lib/i/record.i` does not export that alias, and the migrator currently writes
`Record` for legacy `{..}`. A module that shadows `Record` therefore needs a
manual migration before this rewrite can be relied on. The per-file syntax
marker is also still needed for a function-type guard in a file with no `^`.
The formatter preserves legacy `@main` attributes; removing or relocating an
entry currently requires `scripts/syntax-2-migrate` rather than `i fmt`.
