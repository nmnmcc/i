# SYN2 stage record

Baseline: 2d1cfc56 on lane/syn2. The lane was clean. The study experiments
under target/syn2 were retained as evidence, not installed as production code.
The coordinator explicitly resumed the uncommitted stage-1 documents; they
were kept and the seventh decision was folded into them.

## Sources and interpretations

The six confirmed decisions and coordinator rulings a–i control syntax.
The seventh decision removes @main and selects the default from src/main.i.
The sentence “each may point at any function in any module of the package”
permits private functions in [entries]. Keys use ordinary name syntax; the
values reuse [wasm].exports paths. --entry chooses a named entry; metadata
keeps the selected function path so ABI v3 and both hosts need no new shape.
The sentence “main functions and [entries] of dependency packages have no
effect” restricts all discovery and declaration checking to the root package.
Duplicate binding names and TOML keys replace the old attribute-count error.

Ruling c requires groups around every whole binary/unary match subject.
Ruling d preserves numeric lexing; printers prefer { 1 }.{ ... }.
Ruling f explicitly removes empty tuples: reviewed replacements use Nil;
record forms remain reserved for T3 until its consumers land.
Ruling g requires ordinary shadowable Record lookup. No generated code in
this stage inserts Record; T3 must provide a qualified ordinary exported
alias before its record type printer claims capture-free source output. The hand-off name is
`i/record.All`: an ordinary exported alias defined where the built-in is visible.
Source generators import that module under a fresh local alias and use
`fresh.All`; diagnostics without an import use the canonical identity
`i/record.All`, as existing diagnostics use full module identities. This
requires T3 library integration; no keyword or privileged module lookup is added.

## Prior art

Read actual source before stage 1: Rust parser/expr.rs parse_dot_suffix;
Dart parser_impl.dart parseParenthesizedExpressionFunctionLiteralOrRecordLiteral
and parseParenthesizedExpressionOrRecordLiteral; Gleam parse.rs
parse_case_clause. Take suffix reuse, matching-close lookahead and separate
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

Read refs/heads/lane/a:tests/t3-stages.md and the syntax crate at that ref.
There are no committed structural record AST nodes there yet. The planned
nodes in records §8.3 are ExprKind::Record { base, fields }, RecordField
{ label, label_span, value }, LabelName::{Name, Str}, and TypeKind::Record
{ base, parts }. SYN2 reserves parenthesised records with E0114 until T3
connects those nodes and consumers. The layer must select function suffixes
first, keep call/constructor contexts, and never add a comma to disambiguate.

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

The new surface uses postfix match, `|` for pipe and `^` for an error type.
The default entry is `src/main.i`'s `main`; `[entries]` selects other
functions. During migration the parser still accepts the first surface.
`// syntax: syn1` resolves empty-tuple collisions in old fixtures; it is removed
when those fixtures migrate. Empty tuples require an explicit semantic rewrite.

Record syntax is reserved pending T3: this branch does not invent an AST for
records. T3 owns the record expression and type nodes and their semantics;
the parenthesised surface must construct those same nodes when T3 lands. A
record and a tuple are distinguished by labels or spread, rather than a
trailing comma, and an empty record is `()`.

Prior art: Rust `compiler/rustc_parse/src/parser/expr.rs`,
`parse_dot_suffix_expr` and `parse_dot_suffix`, parse postfix operators in the
same loop as field access and calls, before binary operators. SYN2 similarly
parses `.{` in the postfix loop. The migration tool follows the source-span
approach: it parses the old tree, edits only operator and attribute spans,
retains comments, and groups a compound subject before a postfix match. Its
repository scan reports unresolved empty tuples and entry declarations for
manual handling.
