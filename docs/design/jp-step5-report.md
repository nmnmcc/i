# JP step 5

No new language or library state surface.

Merged redesign into lane/jp, preserving provider plans and the macro/collection
runner, entry selection, HTTP/TLS, task references, CLI/log adapters and editors.
All execution roots use the provider graph and entry context. Selected collection
entries use graph-derived async classification, factory construction and host
registration metadata on both Rust and JavaScript. Test execution preserves the
chosen target column for each selected contribution.

The merged macro driver had no CLI expansion command. Added
`i expand --project PATH src/main.i --to is --trace`, using that same compiler
sandbox, with both output surfaces and explicit errors for Lisp-only forms.

Interpretations and sources:

- Compiler macro programs use an empty sandbox plan through the ordinary planner.
  Their wrappers bind compiler services lexically; user runtime plan declarations
  are outside the macro import closure. This follows meta.md §8: compiler services
  are not host services, no other services are available, and macro programs are
  independent of the eventual program target.
- SYN2 stage A retains the merged dual parser; the selected-entry contract in
  spec.md §10.2/B.6/B.7 is preserved from JP, following the USER ledger decision
  removing @main in favor of src/main.i and [entries].
- The TLS cells were already shipped in lib/provider-plan.toml for native and
  nodejs; no additional capability or compiler service-name rule was necessary.

Prior art: read .rd/ref/INDEX.md and Effect's
packages/effect/src/internal/layer.ts (provideLayer/provide). The existing JP
construction graph follows the same scoped build-then-provide approach; this merge
extends that proven implementation to newly selected tool roots.

Validation results will be recorded after regeneration and acceptance.
