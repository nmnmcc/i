JP step 3: per-entry target provider graphs

API: `ilang_check::providers::resolve_plans(&Checked, &PackageGraph, Target)` returns an entry-keyed map of resolved plans or diagnostics. Plans expose ordered service constructions, factory/host/constructor bindings, generated arguments and service positions, outer/body scope, adapter entry position, roles, request modes, primitive-key need and async classification. The shipped manifest supplies the implemented standard capabilities and ordinary private request/task factories.

Verification uses `H="/home/agent/Programming/i/.rd/cache/heavy direnv exec /home/agent/Programming/i"`.

- `$H cargo check --workspace --all-targets`: passed after both redesign merges.
- `$H cargo check -p ilang-check --all-targets` and `$H cargo check -p ilang-package -p ilang-check --all-targets`: passed.
- `UPDATE=1 $H cargo test --locked -p ilang-check --test types library`: passed; snapshot adds the five private provider factories.
- `$H cargo test --locked -p ilang-check --test provider_graph`: passed: `39 passed; 0 failed; 0 ignored`. Development failures were corrected: generalized adapter proof normalization, closed narrower generic returns, test annotations/expectations, and an adapter type-parameter name collision. The first acceptance attempt printed `test result: FAILED. 11 passed; 25 failed; 0 ignored; 0 measured; 0 filtered out`; normalizing subtraction sets on both sides fixes that regression.
- `$H cargo test --locked -p ilang-check --lib written_occurrences_include_other_constructors_and_aliases -- --nocapture`: initially failed with `top-level binding provider_root is not used` (and the other four helpers). Using the existing private `_` helper convention fixes it; the subsequent crate run passed this test.
- `$H cargo test --locked -p ilang-package -p ilang-check`: initially printed `test result: FAILED. 91 passed; 1 failed; 4 ignored` for those helper warnings; both subsequent full two-crate runs passed, including the final acceptance run (exit 0).
- The second acceptance attempt printed `redesign not merged` after redesign advanced to syntax stage A. Merged `7e240a4e` and reconciled package entries with provider-plan data; workspace check and graph tests passed afterward.

Interpretations: source inference stays before target selection, and the resolver consumes retained entry needs despite the temporary source E0522; existing CLI/compiler E0522 gates remain for step 4 (task item 6; spec §2.6.1). Omitted root metadata inherits, explicit incompatible role changes are errors, and a registered row without a target cell never falls through to custom-host registration (spec §2.6.1). Declared request mode survives factory overrides; WASI retains synchronous classification (spec §10.2/B.7). Standard Host capabilities require explicit cells; native/Node/browser cells follow actual shipped implementations (js.md T3). The old future i/target registry row is omitted because that module does not exist on this branch (task item 5; every declared row must resolve under §2.6.1).

Implementation basis: Effect Layer's MemoMap and ZIO ZLayer's per-build memo map for scope-local sharing; spec §2.6.1 supplies the conservative recursion rule, separate adapter scopes and stable construction order.

Final supplied acceptance script: passed (exit 0), including all 24 required test names, zero ignored graph tests, redesign ancestry, 39 passing graph tests, and both complete crate suites. The final corpus permutation/recovery test passed in 610.16 seconds. `git diff --check` passed.

Branch: `lane/jp`; implementation through `a26bce51`, with this report committed afterward.

Step 3 unfinished work: none. Applying plans in lowering and removing E0522 are step 4, intentionally unchanged.
