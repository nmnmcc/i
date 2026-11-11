# JP step 6

No new language or library state surface.

Migrated this lane with the frozen legacy syntax reader before merging redesign
`a8ce267c`. Named entries retain the positional `i run NAME` / `i build NAME`
interface, and ordinary and collected tests compile into one artifact. Every
collected root constructs its checked provider graph and uses that graph's async
classification and host-registration metadata, with a fresh instance per test.

The merge preserves selected-many inference and capability validation. It removes
the old library-name-based entry dependency installation: construction dependencies
are already included by the provider plans in lowering. The new regression runs a
private named entry and a collected test with a provider whose transitive clock
request requires async execution, at O0 and O2.

Interpretations and sources:

- The latest USER ledger rulings keep `|>` and clause-block matches, remove the
  other old syntax, and choose `[entries]` rows positionally. These override the
  historical syntax study and earlier `--entry` descriptions.
- Provider plans apply unchanged to selected-many test roots (spec §2.6.1/§10.2
  and js.md T3). One-build changes artifact reuse, not per-invocation provisioning
  or fresh test instances.
- The migration tool reports host fragments and deliberate negative fixtures;
  redesign supplies their reviewed migration. New audit classifications cover
  only TOML manifest literals/format templates; the import replacement fragment
  is parsed with its missing header suffix. No i source is classified as host text.

Prior art read: Go `src/cmd/gofmt/rewrite.go` (`rewriteFile`, tree matching and
comment preservation); Gleam `compiler-cli/src/run.rs` (`setup`, root/module
selection); Effect `packages/effect/src/internal/layer.ts` (`provideLayer`, scoped
build then provide); jco `crates/js-component-bindgen/src/lib.rs` (validated
component graph lowered to ordered initializers). The merge extends the existing
JP implementation instead of introducing a second runner mechanism.

Validation records are in `target/jp-*.log` (untracked build output). The final
acceptance results are recorded below after the complete check finishes.

Completed focused checks (all cargo/fixture commands used heavy + direnv):

- `cargo check -p ilang-cli -p ilang-wasm -p ilang-check --all-targets --locked`:
  passed.
- `cargo test -p ilang-cli --test providers --test collections --test entries --locked`:
  passed (17 provider tests, 3 collection tests, 7 entry tests).
- `I=$PWD/target/debug/i scripts/build-abi-fixture.sh`: passed; regenerated
  the conformance binding sidecars for the current trusted library identity.
- `i run --project examples/io-{clock,fs,net} --opt 0`: clock/fs/net ok;
  `examples/http-get`: http ok; `examples/cli-log -- --name world --count 1`:
  last stdout line cli ok.
- `i test --project examples/macro-tests --opt 0`: 8 passed, 0 failed, 1 skipped.
- `i test --project std --opt 0` and `--opt 2`: 1845 passed, 0 failed each.
- `i test --project tests/perf/many-tests --opt 0`: 2000 passed, 0 failed.
- `cd bindings/javascript && direnv exec /home/agent/Programming/i npm test`:
  88 passed, 0 failed.
- `scripts/old-syntax-check`: passed, 4544 extracted literals/blocks, exact
  expected diagnostics, 3 future-track entries, zero findings.

Corrected development failures:

- `scripts/syntax-2-migrate --repo --write` exited 1 after applying 1450 snippet
  changes: `manual: 22`, `invalid: 415`. The reports include deliberate negative
  fixtures and non-i host strings; merged redesign carries their reviewed forms.
- The first syntax audit reported unclassified TOML, for example:
  `crates/ilang-check/tests/provider_graph.rs:27 ... actual ['E0101: expected an item: a type definition, or a binding pattern = value, found [']`.
  Added exact host-literal identities, retained parsing for the import fragment,
  and removed stale manifest identities. Final audit passes.
