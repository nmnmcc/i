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
values use dotted package-relative module paths. --entry chooses a named entry; metadata
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
T3 record nodes. At a clause head, `?` begins a guard; a legacy error type
in that position must be grouped. This is decided by grammar.
The Go `cmd/gofmt/rewrite.go` comment-map approach motivates retaining source
between span edits, and Cargo `src/workspace/parser/targets.rs` and Gleam
`compiler-cli/src/run.rs` distinguish conventional and explicitly selected
entry modules.

## Current migration state

The 10.2 fixture expects E0305 because `both` is not the default entry and
`@test` is outside `test/`. Manifest entry targets are dotted module paths
relative to the package. Clause-head `?` starts a guard by grammar (§4.1).
The formatter removes `@main`; migration declares non-default entries.

For `{..}`, migration keeps the legacy spelling where `Record` appears in
the module, avoiding capture. Empty expressions are accepted only when the
package checker, including tests, succeeds and records an empty-record type
at that span; otherwise they are reported for manual review. The old forms
are listed in `scripts/old-syntax-check.json`.

## Repository migration baseline

On merged redesign 7e240a4e, the WAT_DUMP harness completed successfully and
captured 1,460 spec and package variants in target/syn2-before. The separate
manifest sweep examined 3,914 projects, saved WAT for 3,027 and recorded 19
12-second timeouts; its complete statuses are in target/syn2-before/packages.
Comparison with the migrated tree remains pending.

The migration's reviewed data records exact source identities and diagnostics
for deliberately malformed `.invalid` review inputs and negative spec cases;
the one JavaScript string classified there is host text. The reviewed empty
expressions in the record specs use `()` as an empty record; the negative
`Unit = ()` spec intentionally tests the forbidden type. These decisions
follow spec sections 3 and 6.4 and do not disable reporting for changed inputs.

The merged baseline and migrated spec WAT captures each contain 1,460 variants;
all 1,460 are byte-identical. Focused recaptures preserved source line positions
and selected the declared named entries. The temporarily changed invalid
attribute fixture produced no WAT once its expected error was restored. The
manifest-package sweep and longer retries of baseline timeouts remain in progress.

Stress capture reproduction: `target/debug/i build --offline --opt 0 --wat
--project tests/review/cases/review-k4/perf/wide_generic_1024 --out
 target/syn2-wide1024.wasm` exceeded both the initial 12-second sweep limit and
the 120-second retry while other shared-slot jobs were running. Its archived
baseline built in the longer retry; no migrated WAT was produced in those two
attempts. This is a capture timeout, not evidence of a semantic difference.

The longer baseline retry also timed out at 120 seconds for
`tests/review/cases/review-k4d/perf/mutual100`, `mutual150`, `mutual200`, and
`mutual300`; each had already exceeded the initial 12-second limit. The same
`i build --offline --opt 0 --wat --project <path> --out <file>` command is the
reproduction. `wide_generic_512`, `stress_wide_open_512`, `chain4000`,
`mutual50`, and `runchain4000` have passed the longer paired comparison with
byte-identical WAT.

The review alias-pattern fixture's former entry lived in an intentionally
invalid test module. It cannot be run when that module is loaded, and the
old default build did not load it, so the migrated fixture leaves its `main`
as an ordinary test-module binding. This follows §2.6's root entry selection
and preserves the fixture's default build and negative test behavior.

The paired retry of `review-k4c/cases/stress_wide_open_1024` captured the
baseline but the migrated build timed out at both 12 and 120 seconds. The
baseline `review-k4/perf/wide_generic_2048` also timed out at both limits.
Both use the same `i build --offline --opt 0 --wat --project <path> --out
<file>` reproduction described above.

Final manifest WAT comparison: 3,041 baseline WAT files and 3,043 migrated
files; all 3,039 common files are byte-identical, with no differing common
programs. Two baseline programs have no migrated WAT because their 120-second
retries timed out: `review-k4/perf/wide_generic_1024` and
`review-k4c/cases/stress_wide_open_1024`. Five other baseline stress programs
(`wide_generic_2048` and `mutual100`, `mutual150`, `mutual200`, `mutual300`)
timed out in both baseline attempts, so neither side was compared. Of 19
initial timeouts, 12 paired retries completed byte-identically.

Four migrated programs have newly generated WAT. The three
`review-r2/deep_50`, `deep_200`, and `deep_1000` cases were rejected on the
merged baseline because their annotations lacked `request.Root` and
`request.Owner`; those services were added, and their required checks and runs
now pass. `crates/ilang-check/tests/projects/entry_dependency` was rejected on
the baseline by a legacy `@main` on a non-function in a dependency; that
annotation has been removed, and dependency entries have no effect on the
root package (§2.6).

Entry loading interpretation: a target in `src/` does not require test modules
or dev-dependencies; a target in `test/` does. This follows §2.1 and §2.6 and
keeps unrelated broken test dependencies out of source-entry builds. The deep
R2 review programs retain their explicit effect annotations, extended with the
Root and Owner services required by task operations (§9.2); all three pass
their review checks and runs at O0/O2.

Additional verification: the syntax inventory passed for 4,099 extracted
snippets, with three exact future-track entries. The short generation run, the
six editor tests, the 85 JavaScript tests, and the O0/O2 standard-library
runs (1,836 tests each) passed. The first review regeneration reported the
three deep R2 failures above; their focused retry and the six named-entry
review cases passed.

The repository-wide `scripts/syntax-2-migrate --repo --check` audit reported
zero changes and zero unresolved empty tuples, but exited 1 on 269 unparseable
extracted candidates. Five documentation examples from that report were then
corrected and their targeted migration check passed. The remaining reported
candidates are in Rust/editor/JavaScript test strings or deliberate invalid
fixtures; the separate old-syntax inventory verifies their registered context
and expected diagnostics. Minimal reproduction:
`scripts/syntax-2-migrate --repo --check --use-built-tool`.

After the comparison, `redesign` advanced to `4ed95a05` and was merged into
this lane. The counts above describe the original `7e240a4e` baseline, not
WAT comparisons for the newly merged HTTP and editor work. The new Rust HTTP
fixtures were migrated, and the editor retains both error-type spellings for
legacy parser coverage. The post-merge syntax inventory passes for 4,117
snippets; all seven VS Code editor tests pass.

The migration tool carries a frozen legacy parser in `crates/ilang-syntax/src/parser_legacy.rs`. After bringing the migration tool and its frozen reader into another branch, run `scripts/syntax-2-migrate --repo --write` from that branch.
