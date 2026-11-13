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

Validation (cargo and fixture-generation commands used the prescribed heavy/direnv wrapper):

| Command | Result |
|---|---|
| `cargo check -p ilang-cli -p ilang-macro -p ilang-wasm -p ilang-host -p ilang-check --all-targets --locked` | Pass after resolving the compile errors below |
| `UPDATE=1 cargo test -p ilang-check -p ilang-wasm -p ilang-host -p ilang-macro -p ilang-cli --locked` | Initially failed only at the WASI message assertion below; all earlier targets passed |
| `UPDATE=1 cargo test -p ilang-cli --test spec --locked` | Pass: 410/410 cases at O0 and O2; regenerated 53 trace headers, changing fixture module paths to `spec/main` |
| `cargo test -p ilang-cli --test collections --locked` | Pass; selected factories and transitive requests execute on Rust and JavaScript at O0/O2; current workspace runs also pass the added expansion-command test |
| `cargo test -p ilang-cli --test providers --locked` | Initially failed on the HTTP graph below; rerun passes all 16 tests |
| `cargo test -p ilang-cli --test collections expand_command --locked` | Pass: both surfaces, arguments/result trace, quotation rejection |
| `cargo test -p ilang-cli --test wasi --locked` | Pass: 17 tests after correcting the diagnostic |
| `cargo test -p ilang-host -p ilang-wasm -p ilang-macro --locked` | Pass |
| `I=.../target/debug/i scripts/build-abi-fixture.sh` | Pass; generated Wasm and binding sidecars committed; workspace fixture-current test passes |
| `cargo test --workspace --locked` | Three complete passing runs, including two on the final code; no failing test, panic or compile-error output |
| Provided acceptance block (`bash /tmp/jp-accept.sh`) | Pass, exit 0; both required workspace passes succeed |
| `i run --project examples/io-{clock,fs,net} --opt 0` | `clock ok`, `fs ok`, `net ok` |
| `i run --project examples/http-get --opt 0` | `http ok` |
| `i run --project examples/cli-log --opt 0 -- --name world --count 1` | Last stdout line `cli ok` |
| `i test --project examples/macro-tests --opt 0` | 8 passed, 0 failed, 1 skipped |
| `i test --project std --opt 0` / `--opt 2` | 1845 passed, 0 failed at each level |
| `cd bindings/javascript && direnv exec /home/agent/Programming/i npm test` | 88 passed, 0 failed |
| `i expand --project examples/macro-tests src/main.i --to is --trace` | Pass; expanded module on stdout, invocation arguments/results on stderr |
| Named-entry/private-factory smoke in `/tmp/jp-named-entry` | `i run --entry worker --opt 0` prints `entry ok`; default main is not called |

Corrected failures, with original failing lines:

```text
error[E0063]: missing field `budget_frame` in initializer of `ir::Func`
error[E0599]: no variant, associated function, or constant named `Ilang` found for enum `ir::Target` in the current scope
error[E0308]: mismatched types
src/main.i:7:1: error[E0523]: provider target native entry http_get/main.main: i/net.Net | i/request.Await | i/request.Owner | i/task.Scheduler | i/tls.Tls is not a nominal service
assertion failed: build_error("f = () { 1 }\n").contains("needs a program entry")
error: unrecognized subcommand 'expand'
```

The compile fixes preserve macro accounting, use Host for compiler programs and
Native for runtime test callers, and pass the expanded checked program/types to
the planner. HTTP exposed a shared graph environment: the planner now uses the
checker's ordinary graph view and normalizes the union before enumerating its
nominal service members. The WASI message retains the expected phrase while
explaining selected entries. The absent expansion subcommand is now implemented.
An early smoke using the pre-rebuild CLI reported `E0523 ... UnknownModule` for
macro compiler plans; rebuilding after sandbox-plan isolation resolved it.

The initial workspace formatter check reported formatting diffs, including
`Diff in .../crates/ilang-cli/tests/cli_log.rs:13`; touched Rust files were formatted
with rustfmt. Formatting of unrelated imported files was outside this task.

The named-entry smoke initially used an invalid package-qualified `[entries]`
path and correctly reported `error[E0603]: an entry is name = "module.path.function"`.
Entries are package-relative (`main.worker`), while provider paths are qualified;
this follows spec §2.6 and the existing entry tests. The corrected smoke passes.

All code validation finished on `b1a6e69d`; only this report changed afterwards.
Branch: `lane/jp`. Merged redesign: `df14ea75` (merge commit `a6f4bca1`).
Unfinished task work: none.
