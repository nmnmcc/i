from pathlib import Path
import json,re
r=Path('/tmp/review-a1');root=Path('/home/agent/Programming/i/.rd/rev5')
cs=json.loads((r/'results.json').read_text());runs={v['id']:v for v in json.loads((r/'runtime-results.json').read_text())}; nruns={v['id']:v for v in json.loads((r/'runtime-new-results.json').read_text())}
report='''# Independent A1 review — 7f60eb3 against b28fca5

**Verdict: do not base the A2 mass migration on this head yet.** The migration can successfully overwrite a valid program with one that no longer checks. Its write validation also has a demonstrated false equality. Fix those major issues first, along with the layout and idempotence failures below. Harden the allowed-mode WAT comparator before treating its results as independent evidence.

I read `docs/design/syntax.md`, then `docs/spec.md`, then `docs/design/a1-front-end.md`, and froze the criteria and 66 programs before inspecting implementation. Original records: [criteria](/tmp/review-a1/criteria.md), [programs](/tmp/review-a1/programs-before-implementation.md), [machine-readable corpus](/tmp/review-a1/corpus.json). Subsequent adversarial cases are explicitly separate below. No source changes or commits were made.

No running, successfully migrated program in this sample produced a silently different result, and no panic was observed. That does not negate the demonstrated broken migration or false-equality checks. Runtime assertions cover specific inputs, not universal equivalence.

## (a) Acceptance criteria and execution method

'''
report+='\n'.join((r/'criteria.md').read_text().splitlines()[2:])+'\n'
report+='''
The normative replacement language is the authority; implementation notes do not redefine it. E0113/J1 and the E0235 choices deferral are treated as documented transitional limitations, not full new-language compliance. D8 also explicitly documents E0236 for introducing a new lowercase variable in a binding that declares uppercase variables.

The corpus has 28 old, 35 new and 3 mixed programs. Each was parsed, checked with the real `i check --types`, and passed to the real `i fmt` (`--from-old` for old programs). Successful output was reparsed, checked, and tested with `i fmt --check`. A library probe additionally checked the syntax printer's strict round trip. Every successful format in this original 66-case corpus was idempotent; the later adversarial case below was not.

For all 28 old programs I added an `@main` assertion and compiled/executed the original through `parse_module_old`, the normal resolver/checker/lowering and host. This matters for the three originals that Both mode rejects. All 28 old baselines passed. The real CLI migrated 26, whose new executions passed the identical assertions; cases 18 and 21 were safely refused. I also executed before/after assertions for all 21 accepted new/mixed originals. Negative programs retain their expected rejection. Case 29 was a mistake in my initial expected result; its original is preserved, and a separate corrected version passes.

Per-case WAT comparisons were also run where both sides compiled. Most original modules have no entry and some functions may be eliminated, so I do **not** use those equalities alone as evidence of their behavior. The assertion-bearing packages make the tested paths reachable. Runtime evidence is in [runtime-results.json](/tmp/review-a1/runtime-results.json), [runtime-cli-results.json](/tmp/review-a1/runtime-cli-results.json), and [runtime-new-results.json](/tmp/review-a1/runtime-new-results.json). Full parse/check/format diagnostics and emitted texts are in [results.json](/tmp/review-a1/results.json).

All runs used the normal compiler with the equivalence feature disabled. `i run --opt 0` was used for the migration regressions. The active blind suite ran at both optimization levels 0 and 2.

## (b) Findings

### F1 — Major: generated function binding captures an imported module

**Ran:** original CLI check/run succeed; both plain `i fmt` and `i fmt --from-old` exit 0 and write output; subsequent CLI check/run fail with two E0207 diagnostics, “`g` is a value, not a module.” This is not an E0113-only old file: the original also checks in Both mode.

Location: [migrate.rs:499](/home/agent/Programming/i/.rd/rev5/crates/ilang-syntax/src/migrate.rs:499), [migrate.rs:781](/home/agent/Programming/i/.rd/rev5/crates/ilang-syntax/src/migrate.rs:781). `expression_names` records the final identifier of an expression name, but omits its module qualifier. R5 then considers `g` fresh even though `g.Box` uses it. The inserted function binding is recursive/in scope in its body and hides the module.

Package `app`, `src/types.i`:

```i
export (Box(..))
Box { Box(Int) }
```

`src/main.i`:

```i
import (g = app/types)
f = () { case (x) -> Int { case g.Box(x) { g.Box(v) -> v } }(42) { _ -> 1 } * 10 }

@main
main = () { assert(f() == 10) Nil }
```

Output contains:

```i
g = (x) -> Int {
    g.Box(x) |> {
        g.Box(v) -> v
    }
}
```

The syntax-only write check accepts the capture because the plan itself contains the wrong binding. R5 requires the generated name not to be used in the relevant block. Fix fresh-name collection to include module qualifiers in expressions, patterns and types, or use a resolver-aware hygiene check. Add a regression with a real imported module graph. The existing ownership test's synthetic graph does not validate such imports.

Evidence: [capture original](/tmp/review-a1/confirmed/capture-from-old/original.i), [first output](/tmp/review-a1/confirmed/capture-from-old/once.i), [CLI results](/tmp/review-a1/confirm-results.json). Run `python3 /tmp/review-a1/confirm.py` from the review worktree to regenerate the CLI evidence in scratch.

### F2 — Major: the production write check accepts changed string values

**Ran:** the following check returns `Ok(())`; the two canonical shapes compare equal.

```rust
let old = "f = () { \\\"@1..2\\\" }\\n";
let out = "f = () { \\\"@3..4\\\" }\\n";
let (module, errors) = ilang_syntax::parse_module_old(old);
assert!(errors.is_empty());
let plan = ilang_syntax::migrate::plan(
    &module, &ilang_syntax::migrate::Env::new(&module));
println!("{:?}", ilang_syntax::migrate::check_written(&module, &plan, out));
```

Location: [shape.rs:10](/home/agent/Programming/i/.rd/rev5/crates/ilang-syntax/src/shape.rs:10), used by [migrate.rs:1313](/home/agent/Programming/i/.rd/rev5/crates/ilang-syntax/src/migrate.rs:1313). Span removal scans the debug string without recognizing quoted payloads, so it deletes `@1..2` and `@3..4` inside string literals as if they were source spans. This helper existed as a test technique, but this track newly relies on it for production migration validation.

This is a demonstrated hole in the stated write-check guarantee. I did **not** observe the formatter itself changing one literal into the other. Fix by comparing structured AST data with spans normalized structurally, preserving literal and comment/doc payloads. Add negative tests that change literal content.

Evidence and exact executable source: [shape-counter.rs](/tmp/review-a1/shape-counter.rs); run `/tmp/review-a1/shape-counter`.

### F3 — Major: `--keep-layout` drops a required closing brace

**Ran:** this valid old program checks and runs. `i fmt --keep-layout --report src/main.i` exits 1 with E0010 and E0101 (“`{` is not closed” / “expected a closed `{`”), despite there being no such error in the original. The CLI leaves the original unchanged. The plan reports `g-binding` and `one-clause`, with no intended hand-edit refusal.

```i
f = () { case (x) -> Int { x }(42) { _ -> 42 } * 10 }

@main
main = () { assert(f() == 420) Nil }
```

Location: [migrate.rs:1496](/home/agent/Programming/i/.rd/rev5/crates/ilang-syntax/src/migrate.rs:1496), [migrate.rs:1647](/home/agent/Programming/i/.rd/rev5/crates/ilang-syntax/src/migrate.rs:1647), [migrate.rs:1709](/home/agent/Programming/i/.rd/rev5/crates/ilang-syntax/src/migrate.rs:1709).

The precedence group and function-extraction block each need their own ` }` at the same offset. They have distinct ordering values (10 and 100). The final deduplication compares only span and text and removes one. Instrumenting a **scratch copy** of the splicer confirmed its malformed intermediate text:

```i
f = () { { g = (x) -> Int { x }
{  g(42) |> { _ -> 42 } } * 10 }
```

Fix edit identity/composition so distinct insertions at the same point retain their multiplicity and ordering. This blocks the layout-preserving migration path needed for layout-sensitive fixtures; it does not corrupt the input on disk.

Evidence: [original](/tmp/review-a1/confirmed/idempotence-keep-layout/original.i), [CLI results](/tmp/review-a1/confirm-results.json), [instrumented splice log](/tmp/review-a1/splice-debug.log).

### F4 — Minor: successful canonical formatting is not idempotent

**Ran:** use the same valid program as F3. Plain `i fmt` and `i fmt --from-old` exit 0, but immediately running `i fmt --check` exits 1. The first output ends the expression with `} *` followed by an indented `10` on the next line; the second changes it to `} * 10`. Both executions still return the expected result 420.

Location: the extraction rendering branch, [format.rs:1450](/home/agent/Programming/i/.rd/rev5/crates/ilang-syntax/src/format.rs:1450). It constructs a synthetic block document separately from the ordinary block/group formatting path; this explanation is based on source inspection, while the non-idempotence itself was directly executed. Make extraction rendering reach the same formatting fixed point as the parsed result.

Evidence: [first output](/tmp/review-a1/confirmed/idempotence-from-old/once.i), [CLI results with first and second text](/tmp/review-a1/confirm-results.json). A further 1,200 grammar combinations found 96 formatting fixed-point failures in this family; those include parser-valid but unchecked inputs, so severity and the claim above rest on this checked/running reproducer.

### F5 — Minor: allowed-mode WAT comparison can report different modules equal

**Ran and validated both modules:** A exports `answer()` returning 11; B returns 22. An empty allowed file grants no exceptions, but:

```sh
python3 -I scripts/wat-diff /tmp/review-a1/wat-counter/a /tmp/review-a1/wat-counter/b --allowed /tmp/review-a1/wat-counter/allowed
```

exits 0 and prints `WAT equal: 1 shared exports, 0 blind exports`. The same comparison **without** `--allowed` correctly exits 1. `/tmp/review-a1/wat-run` parses, validates and executes both WAT files using Wasmtime.

Location: [scripts/wat-diff:247](/home/agent/Programming/i/.rd/rev5/scripts/wat-diff:247), [scripts/wat-diff:260](/home/agent/Programming/i/.rd/rev5/scripts/wat-diff:260). Allowed-mode normalization discards function order across owners, but only renames symbolic function references. Numeric references retain their old indices. With `(export "answer" (func 1))`, moving definitions changes which function is exported while normalized representations stay equal.

A is:

```wat
(module
(type $sites (array (mut i32)))
(type $values (array (mut i32)))
(data $site_data "")
(func $sites_install (param (ref $sites)) (param (ref $values)))
;; app/main.a
(func $f0 (result i32) (i32.const 11))
;; app/main.b
(func $f1 (result i32) (i32.const 22))
(func $ilang_start
(call $sites_install (array.new_data $sites $site_data (i32.const 0) (i32.const 0))
(array.new_fixed $values 0)))
(export "answer" (func 1))
)
```

B only swaps the two commented function definitions. Files: [A](/tmp/review-a1/wat-counter/a/test.wat), [B](/tmp/review-a1/wat-counter/b/test.wat), [execution helper](/tmp/review-a1/wat-run.rs).

Scope matters: the current compiler emits symbolic function references, so this is **not** evidence of a miscomparison on its current output dialect. It is a concrete false equality on valid WAT accepted by the utility. Reject numeric function references in allowed mode unless they are resolved to stable identities. A safety comparator restricted to emitted WAT should explicitly reject unsupported forms.

## (c) Diff audit and unsupported expectations

The two forbidden files, `crates/ilang-check/src/types/infer.rs` and `crates/ilang-wasm/src/lower.rs`, are byte-unchanged between base and head (`git diff --exit-code` succeeds). The changed paths fit the track's documented scope, including its explicitly listed resolver/query/CLI/test/script exceptions. I found no unrelated source edits. [Full changed-path list](/tmp/review-a1/changed-paths.txt).

The replacement language text in `syntax.md` before “Notes for A1 and A2” is byte-identical to base. Documentation edits there change implementation notes and handoffs. The 96 files in `golden_old` are byte-identical to their base `golden` counterparts. Both-mode golden changes have reasons: reserved-form errors move to the appropriate parser/checker stage, empty braces acquire clause semantics, attributes are checked as calls, headers/type parameters gain the new diagnostics, and formatter outputs use migrated headers/types/match groups. I found no unexplained golden adjustment.

The strict parsed-tree round-trip assertions remain. Formatting tests now compare against the migration plan and maintain an explicit skip list; those changes are necessary for the changed contract, and do not silently remove all checks for refused inputs. The structural equality weakness is F2. Existing passing ownership tests are insufficient for F1's imported-module case. I found no additional unjustified test weakening.

All **37 active blind cases** have semantic expectations supported by the language/spec; all passed at `--opt 0` and `--opt 2`. Their exact E/W numbers are implementation choices, not language requirements. The expectation audit is itemized below.

Two original blind expectations were not required by any source, and the track correctly corrected them:

- Rejecting a private `@main`: §10.2 restricts host exports to the exports list, not the program entry. The merged test correctly expects success.
- Accepting the original explicitly typed `outer` while it calls `assert` and omits `! Assert`: §6.8 says omitted effects in an explicit arrow part mean `Never`; §9.6 gives `assert` the `Assert` environment. The merged case correctly adds `! Assert`.

The pending one-clause choices case is a language-required success (§4.8/§6.12), deliberately not an active passing A1 claim. E0235 is the documented T1 deferral. It must not be cited as implemented success.

My own frozen case 29 originally expected success, but its tuple starts on the same line as a function expression. §1.7 reads that as a call, so the E0105 rejection is correct. Adding a newline before the tuple makes it check, format idempotently, and run with `outer(True) == (True, 42, "s")`. This is a review-input correction, not a compiler finding. [Corrected source](/tmp/review-a1/corrected29/src/main.i), [results](/tmp/review-a1/corrected29-results.json).

## (d) A2 decision and validation

A2 should wait for F1 and F2 to be fixed: they invalidate migration safety assumptions. Fix F3 before using `--keep-layout` on the layout-sensitive A2 partition, and F4 before relying on a single formatting pass/clean `fmt --check`. Add F5's unsupported-input rejection before using allowed-mode equality as an unchecked safety assertion. Then rerun these regressions and the actual A2 source corpus; preserve the documented hand-edit refusals and pending T1 limitation.

Checks completed successfully, despite these independently reproduced defects:

- `cargo build -p ilang-cli`.
- `cargo test -p ilang-syntax -p ilang-check`: [log](/tmp/review-a1/cargo-tests.log).
- `cargo test -p ilang-cli --test check --test fmt`: 7 check + 4 fmt tests, [log](/tmp/review-a1/cli-tests.log).
- `cargo test -p ilang-cli --test spec -- a1_`: all 37 active cases at optimization 0 and 2, [log](/tmp/review-a1/blind-tests.log).
- `python3 -I scripts/wat-diff-test`: [log](/tmp/review-a1/wat-diff-test.log).
- The 66 independent cases, 49 original assertion-bearing baselines (28 old + 21 new/mixed), 47 successful migrated assertion executions, corrected case 29, and the concrete regressions above. The two excluded post-migration executions are the explicit safe refusals.

I did not run the entire workspace/dev-verify or perform an A2 mass rewrite. No exhaustive equivalence claim is made.

## (e) Environment and restoration

All cargo commands used the prescribed heavy/direnv wrapper, from this worktree, with `CARGO_TARGET_DIR=/tmp/review-a1/target`. Tool binaries, probes, test packages, logs, instrumented copies, and report artifacts are under `/tmp/review-a1/`. Compiler instrumentation was confined to a copied syntax crate in scratch; repository source was never instrumented. The initial probe-link attempt failed, then succeeded using the environment's toolchain PATH; no toolchain configuration was changed.

No files in another repository directory were deliberately edited. The prescribed direnv startup ran its own `devenv:files:cleanup` and `enterShell` tasks; I did not modify their configuration or attempt to undo tool-managed caches. Environment variables were per-process. No persistent environment changes required restoration. No commits, index changes, branches, or worktrees were created. Final tracked/untracked status in the assigned worktree is clean; scratch evidence is intentionally retained.

## Blind-spec expectation ledger

Every active row below passed at both optimization levels. “Pending” is excluded from the active runner and is not counted as a pass.

| File under tests/spec | Expectation | Source justification |
|---|---|---|
'''
# Fill from actual headers, with independent justifications.
why={
'a1_line_breaks':'§1.7: continuation, line-separated tuple/list parts, negative and tuple clause boundaries.',
'a1_same_line_negative':'§1.7: same-line minus continues the preceding expression.',
'a1_same_line_tuple':'§1.7: same-line parentheses continue as a call.',
'a1_entry_exports':'Spec §10.2: @main need not be exported; host exports must be.',
'a1_identity_recursion_tail':'§6.10/6.12 direct functions; §11.4 identity; §11.6 tail recursion/matches.',
'a1_builtin_attribute_call':'§2.2: a zero-argument built-in attribute has no parentheses.',
'a1_empty_header':'§2.2: imports and exports groups are nonempty.',
'a1_header_order':'§2.2: imports precede exports.',
'a1_headers_tuple':'§2.2–2.5: slash identifies imports; = identifies a tuple binding; direct mutual functions.',
'a1_macro_attribute_line':'§2.2: unknown attributes are kept for macros; newline leaves the tuple item separate.',
'a1_macro_attribute_same_line':'§2.2/§1.7: parentheses on the attribute line are its arguments, leaving an invalid item.',
'a1_macro_declaration':'§2.2/4.7: declaration without @external is reserved for macros.',
'a1_macro_item':'§2.2: top-level call is reserved for macros.',
'a1_macro_type_call':'§3: lowercase call in a type is reserved for macros.',
'a1_variable_arguments':'§3/4.7: a type variable takes no arguments.',
'a1_alias_pattern':'§4.3: alias head not a parameter list is reserved for alias patterns.',
'a1_custom_type_head':'§4.3: type parameter may not be a visible type name (Int).',
'a1_declared_variables':'§4.7: explicit variables, inner shadowing, inherited scope; §4.3 aliases; §6.8/9.6 Assert effect.',
'a1_duplicate_variables':'§4.7: declarations in one place have distinct names.',
'a1_undeclared_variable':'§4.7: unknown uppercase type name must be declared.',
'a1_variable_hides_type':'§4.7: a declared variable cannot hide a visible type.',
'a1_assert_required':'§4.8/6.12: refutable one-clause fitting introduces Assert, incompatible with a pure type.',
'a1_brace_decisions':'§6.1 ordered first-statement tests, alias patterns and function-binding exception.',
'a1_empty_inhabited':'§4.8 rule 11/6.12: empty clause function accepts Never, not Int.',
'a1_empty_never':'§4.8 rule 11/6.12: empty clause function accepts Never.',
'a1_expected_argument':'§4.8/6.12: expected function type reaches clauses and known field types.',
'a1_fitting_expected_subject':'§4.8/6.12: fitting shares expected/known types and adds Assert for refutable patterns.',
'a1_labels_and_catchall':'§6.12 single binder label; §4.8 rule 11 generic catch-all; §6.1 function-type grouping.',
'a1_no_label':'§6.12: multi-clause parameter has no label.',
'a1_one_clause_choices':'§4.8/6.12 require fitting choices together; D7 defers to T1 with E0235.',
'a1_opening':'Opening/§6.1: function body always a block; §7.1: multi-clause annotations are tests.',
'a1_pipe_groups':'§6.9 precedence plus groups around match operands, callees and field targets.',
'a1_record_kept':'§6.1: name-colon form with no legal pattern delimiter is reserved for records.',
'a1_unused_clause':'§6.10/6.12: unused function value warns.',
'a1_wrong_pattern':'§6.1: malformed first clause pattern is an error, not a fallback block.',
'a1_coverage_guard':'§6.12/7.4: guarded clauses do not count toward coverage.',
'a1_no_subject_constructor':'§6.12/7.5: fitting does not narrow the subject to a known constructor.',
'a1_one_clause_annotation':'Opening/§6.12/7.1: single unguarded clause annotation fits rather than tests.',
}
for p in sorted((root/'tests/spec').rglob('a1_*.i*')):
 s=p.read_text(); name=p.name.split('.i')[0];expect=re.search(r'// expect: (.*)',s).group(1)
 if p.suffix=='.pending':expect='Pending: '+expect
 report+=f"| `{p.relative_to(root/'tests/spec')}` | {expect} | {why[name]} |\n"
report+='''
## Original 66 programs, with individual results

The exact original bytes are shown below. Each program lives in its own `app` package. Parse means Both mode unless an Old result is explicitly stated. Check includes parse diagnostics; warning-only results still succeed. For a successful format, its post-check result is shown and `fmt --check` was run. An error-case formatter may succeed because it promises syntactic validity, not successful type checking.

The runtime predicate shown for a case was put into `@main main = () { assert(PREDICATE) Nil }` in a separate executable copy. It passed both before and after formatting unless explicitly marked as a migration refusal. Raw originals were preserved.

'''
for c in cs:
 b=json.loads(c['before']['out']);report+=f"### {c['id']:02d}. {c['name']} — {c['kind']}\n\n```i\n{c['source']}```\n\n"
 report+=f"Frozen expectation: {c['expected']}.\n\n"
 def diag(ds):return '; '.join(ds) if ds else 'OK'
 # avoid long repeated E0113 message; exact stored separately
 pd='; '.join(x.split(':')[0] for x in b['parse']) or 'OK'
 cd=diag(b['check']) if not b['parse'] else pd+' (parse rejection)'
 report+=f"Result: parse **{pd}**; check **{cd}**. "
 if 'old' in c:
  ob=json.loads(c['old']['out']);report+=f"Old parse **{'OK' if not ob['parse'] else diag(ob['parse'])}**, Old check **{diag(ob['check'])}**. "
 if c['fmt']['rc']==0:
  a=json.loads(c['after']['out']);ad=diag(a['check']);report+=f"CLI format **OK**; output parse **OK**, output check **{ad}**; idempotence **{'OK' if c['idempotent']['rc']==0 else 'FAIL'}**."
 else:report+='CLI format **refused**'+(' (`fn-expected-type`, intended safe refusal).' if c['id'] in (18,21) else ' (parse errors; no output written).')
 report+='\n\n'
 rt=runs.get(c['id']) or nruns.get(c['id'])
 if rt:
  report+=f"Executed predicate: `{rt['assertion']}`. "
  report+=('Old baseline passes; migration intentionally refused, so no migrated execution.\n\n' if c['id'] in (18,21) else 'Passes before and after real CLI formatting.\n\n')
 if c['id']==29: report+='The frozen success expectation was wrong: same-line `(` is a call (§1.7). Separate newline-corrected follow-up checks, formats idempotently, and passes its before/after runtime assertion; see section (c).\n\n'
 if c['id']==38:report+='This is the documented T1 deferral, not satisfaction of the full language rule.\n\n'
 if c['id']==33:report+='The intended rejection occurs in parsing: lowercase `f(x)` is a call, not a constructor pattern. No claim depends on the exact diagnostic phase.\n\n'
(r/'report.md').write_text(report)
print('Wrote',len(report),'characters to',r/'report.md')
