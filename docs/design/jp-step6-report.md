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
