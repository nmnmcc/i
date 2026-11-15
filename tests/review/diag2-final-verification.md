# Diagnostics lane verification

NOT YET LANDABLE: runtime verification queued.

Branch: `lane/diag2`. Source commit: `e13498261ff57e4f519381140f955418b4099d0b`. Main `a02d34100` was merged as `eed4a9ef01679ab525d80d5e1c176282ac75baa7`. The one conflict, `scripts/check-snippets.json`, kept main’s complete registry and reapplied exactly the lane’s three entries (two bang diagnostics and TOML-literal classification). Main’s removed stale entries stayed removed.

The five original repairs and inferred library signatures were fully verified at `4bbfc4b7b` after that merge: 2,171 std tests, 268 filtered spec cases at both opt levels, clippy, release build, formatter checks, 5,248 snippets, 61 corpus cases, and all docs checks passed. All 75 E4 signatures were audited. Those are historical results, not fresh verification of the additions.

The additions remove documentation references from 118 message classes at 158 source sites and add two contextual argument-message classes. The original 169 marks comprise 128 emitted-message marks, seven expected-message marks, four ignored-test-description marks, and 30 embedded program/template marks. After main, the corresponding total is 189. All diagnostic references are gone; the 36 retained marks are in exact embedded programs/templates and four ignored-test descriptions. All eight archived inputs remain exact.

E0501 remains on the rejected argument. E0552 is dropped only when this call’s own argument constraint reports E0501. Every constraint still runs. Independent errors and E0552 for an unmatched combination remain. The new tests cover integers, strings, later and piped arguments, Boolean/data arguments, independent errors inside an argument, two bad arguments, a still-rejected combination, and an accepted matching call.

Complete before/after wording and source counts: [wording report](diag2-diagnostic-wording.md) and [literal audit](diag2-diagnostic-wording.json). Exact corpus before/after text and counts: [771-case proof](diag2-final-corpus-proof.json); the complete [case list](diag2-final-cases.json) has 3,302 message-only changes across 124 rendered classes. Codes, positions, counts, stdout, statuses, phase commands, bounds and observation metadata remain exact. No existing corpus record contains the E0552 pair; new CLI tests and the queued isolated capture verify the dropped duplicate.

The original archive paths and hashes are in the literal audit and `crates/ilang-cli/tests/fixtures/diagnostic_repairs/README.md`. Three new CLI tests are in `crates/ilang-cli/tests/diagnostic_repairs.rs`; its archive test additionally rejects section references. Existing diagnostic, literal-type and dependency-name assertions keep their rule-checking intent. No archived program or existing test is removed.

## Commands and current results

All heavy work uses `/home/agent/Programming/i/.rd/cache/heavy`. The focused script holds one slot and runs commands sequentially. Rust test harnesses use two threads. Broad commands check for a running gate before every command. Docs set `I_BIN` to this tree’s release compiler and `I_SNIPPET_PARSER` to its existing debug parser.

| Command | Status | Last nonempty line |
| --- | --- | --- |
| cargo test -p ilang-cli --test diagnostic_repairs --release --locked -- a_literal_outside_all_branches_gets_one_argument_error | queued | (no output) |
| cargo test -p ilang-cli --test diagnostic_repairs --release --locked -- an_unmatched_combination_still_reports_branch_selection | queued | (no output) |
| cargo test -p ilang-cli --test diagnostic_repairs --release --locked -- unmatched_types_keep_grouping_advice_and_independent_argument_errors | queued | (no output) |
| cargo test -p ilang-cli --test diagnostic_repairs --release --locked -- original_model_programs_get_actionable_repairs | queued | (no output) |
| cargo test -p ilang-cli --test package_commands --release --locked -- add_checks_the_names | queued | (no output) |
| cargo test -p ilang-check --test diagnostic_traps --release --locked -- two_spaced_constructor_parts_are_still_one_binding | queued | (no output) |
| cargo test -p ilang-check --test diagnostic_traps --release --locked -- spaced_function_call_keeps_its_error_and_explains_the_line | queued | (no output) |
| cargo test -p ilang-check --test diagnostic_traps --release --locked -- missing_known_parameter_type_explains_both_fixes | queued | (no output) |
| cargo test -p ilang-check --test literal_types --release --locked -- empty_optional_accumulator_requires_a_written_tuple_shape | queued | (no output) |
| cargo test -p ilang-cli --test spec --release --locked -- 6/ | queued | (no output) |
| scripts/review-corpus run --binary ./target/release/i --filter review-t1/service_bad --keep-bounds --jobs 1 | queued | (no output) |
| python3 /tmp/diag2-final-call-proof.py | queued | (no output) |
| scripts/review-corpus run --binary ./target/release/i --filter review-t1/host_service --keep-bounds --jobs 1 | queued | (no output) |
| direnv exec /home/agent/Programming/i cargo fmt --all -- --check | queued | (no output) |
| direnv exec /home/agent/Programming/i cargo check -p ilang-check --all-targets --locked | queued | (no output) |
| direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test diagnostic_repairs --release --locked | queued | (no output) |
| direnv exec /home/agent/Programming/i cargo clippy --workspace --all-targets --locked -- -D warnings | queued | (no output) |
| direnv exec /home/agent/Programming/i cargo build --release -p ilang-cli --locked | queued | (no output) |
| scripts/check-snippets | queued | (no output) |
| scripts/review-corpus run --binary ./target/release/i --case-list tests/review/diag2-final-cases.json --keep-bounds --jobs 1 | queued | (no output) |
| direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test spec --release --locked -- 2.4/ | queued | (no output) |
| direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test spec --release --locked -- 4.3/ | queued | (no output) |
| direnv exec /home/agent/Programming/i cargo test -p ilang-cli --test spec --release --locked -- 9/ | queued | (no output) |
| ./target/release/i fmt --check lib std bench examples | queued | (no output) |
| ./target/release/i test --project std --opt 0 | queued | (no output) |
| devenv shell -- bash -c 'cd docs && exec npm run generate' | queued | (no output) |
| heavy_docs npm exec -- astro build | queued | (no output) |
| devenv shell -- bash -c 'cd docs && exec npm run check:placeholders' | queued | (no output) |
| devenv shell -- bash -c 'cd docs && exec npm run check:links' | queued | (no output) |
| devenv shell -- bash -c 'cd docs && exec npm run check:rendering' | queued | (no output) |
| devenv shell -- bash -c 'cd docs && exec python3 scripts/check-examples.py' | queued | (no output) |
| heavy_docs python3 scripts/run-examples.py --no-placeholders --jobs 1 | queued | (no output) |

Static verification commands passed: the provided Rust-literal lexer audit (`python3 /tmp/diag2-final-static.py`), full corpus header/non-message comparison against main (`python3 /tmp/diag2-final-aggregate.py`), rendered-class count reconciliation, and `git diff --check`. Formatting was applied with `devenv shell -- cargo fmt --all`; the explicit check remains in the queue above. The lexer audit’s last line was: “PASS: zero diagnostic section references; all 36 remaining marks belong to 11 exact embedded programs/templates and four ignored-test descriptions; eight original archive inputs unchanged; lib/std/examples/docs content identical to main.” The corpus audit’s last line was: “771 records versus a02d34100; 3302 message-only changes; no count changes in the existing corpus; the new CLI tests cover duplicate counts.”

The preliminary release test failed with exit 101 because its expected string omitted the CLI’s existing `1 error` footer. Its actual primary diagnostic already matched the desired E0501 at `2:16`; the footer expectation is corrected. Its last line was “error: test failed, to rerun pass `-p ilang-cli --test diagnostic_repairs`”. The preliminary debug invocation and later queue-refresh invocations were canceled before execution.

Full workspace tests, unfiltered spec/corpus runs and dev-verify remain coordinator-owned. Current pending results above are not claimed as green. The shared slot initially held the batch; after g30 ended, the wrapper’s load guard held it while one-minute load was above 28 (86 observed).

## Performance cases rerun alone

None on this lane so far. No own corpus phase has timed out. The coordinator’s g30 had three timeouts on its tree; none of those cases is in this lane’s selected 771-case rerun. Any timeout in an own run must be measured on a pinned core for three runs on main and this tree before it can be classified.
