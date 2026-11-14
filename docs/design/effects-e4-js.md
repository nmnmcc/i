# E4 JavaScript runtime follow-up

The lane first merges main-integrated E3/E5/E6 `7dce741db` (merge
`8e148f52c`). No JavaScript ABI signature or error-channel convention changes
are needed for the two reported traps.

## Host export after an ordinary error

A fresh `app/app.distance(3, 4)` succeeds. After `app/app.lookup("other")`
returns its ordinary String error, the same distance call traps at
`request_entry_begin`, before calling `hypot`. The ABI remains
`hypot(f64, f64) -> f64`; the failure is a live entry context, not Float
encoding. The compiled test already checks that the instance stays usable.
It now also repeats a successful lookup after the distance call.

The entry context uses `provider_finish` to catch errors, end its request
context, and raise the same error to the host. Its inner stub and the closure
calling it used `Env::Root`, bypassing the catch's provided Fail. Both now use
`Env::Param`: the catch reaches the inner stub, platform services are installed
on that environment, and the completion code runs before the error goes out.

Spec §9.3: "An error goes out through the calls to the `catch` that provided
its `Fail`. The computation that is jumped over is unwound first (§9.5)."
Spec §10.2: "For a host export, it takes only an error type that the host can
take, and the error is given to the host (Appendix B)."

## Streaming fetch after a suspension

WAT builds of the current std and a pre-E4 std snapshot (`d5d8e9042`) both
reproduce the same failure. `std/http.client` (`f157`) has the same `$fn0`
signature, reads service slot 10, and calls `$field` to obtain `Client.make`.
The request has already suspended on its first streamed read. When the caller
makes its second HTTP request, slot 10 is empty in the resumed environment.
`field` expects an object and receives that empty slot.

The fetch test's `checked` adapter provided Client in a tail call:

```i
with_client = (body) {
  fail.catch(() { body() <- (http.fetch_client()) })
}
```

Its WAT ended in `return_call_ref`, so the provision had no surviving frame
and no delta to replay. This is the existing general tail-provide gap
explained by `lazy-reentry.md` D16 and §7a. The provider adapter now follows the
same retained-call pattern as the library adapters:

```i
with_client = (body) {
  fail.catch(() {
    value = body() <- (http.fetch_client())
    value
  })
}
```

At O0 and O2 the WAT contains a capture record of kind 1 with the Client delta
`[10, Client]`, and its re-entry rebuilds that provision. Both streaming probes
return `"stream ok"`, including all first/second/end-of-body assertions. The
complete fetch suite now runs at both levels; no assertion changes.

Spec §9.4 rule 3: "The paused part keeps the services provided inside it; for
the rest, it gets the services of the place where `resume` is called."
`runtime.md` §6.3 explicitly says: "Use a non-tail call after those environment
statements, through a surviving identity frame when necessary." This change
applies that existing adapter rule; it does not claim to fix the compiler's
general tail-provide gap or change which calls are in tail position (§11.6).

## Investigation artifacts and validation

Reproductions live in ignored `target/e4-js-debug/`. The pre-E4 std snapshot is
an archive inside this worktree; no branch or other checkout is changed. Each
module is built with `i build --target js --opt 0 --wat`; the corrected fetch
adapter is also built with `--opt 2 --wat`. `wasm-tools print` is run on the
before/after binaries. Both the compiler WAT and the printed binary confirm
that the ABI signatures are unchanged. The relevant difference is the
preserved environment and completion path.

- Release build after the merge: pass; release rebuild after the lowering fix: pass.
- Baseline probes: distance after an error traps; streaming traps with both std snapshots.
- Corrected compiled fixture: all 8 JavaScript tests pass, including exact stop positions.
- Corrected streaming probes: pass at O0 and O2.
- Complete ABI regeneration and Wasm validation pass, including the R2 generator.
  All five generated binding plans retain their contracts after digest normalization.
- First full JS run: 108/117 Node checks pass. Seven IO fixtures still build
  for `host`, and two fixtures still use the removed manifest provider table.
  The IO builds now select `nodejs`; the two fixtures provide their chosen
  services in their entry bodies. All their assertions are retained. Spec
  §2.6.1: "Every other service is given in the entry by i code." It also says:
  "There is no `providers` key in the package file."
- The generation driver emitted legacy clause blocks, type declarations,
  function heads and service statements. Its source strings are ported to the
  current syntax. All model inputs, coverage decisions and expected results
  match the previous driver for 720 short/long seeded cases; no oracle changes.
  The emitted chain preserves branch order and guards under spec §6.12.
- The second Node run passes 114/117. Its three remaining failures are
  explained: the first provider's exported function now needs its local Host
  type, so that type is exported (§2.4); the fake-clock fixture adds a task
  yield to keep exercising async request mode under the new need-based
  classification (§10.2); the process example's old machine-wide check sees
  another worker's sleep child. The latter now checks only children whose
  parent is this run's host process, preserving both cancellation assertions.
  The Node test deliberately keeps an unrelated sleep child alive while
  checking this, and terminates only that test-owned child afterward.
- Short generation gate passes: 32 cases (20 accepted, 12 model rejections),
  64 declaration permutations, 32 annotation removals, no failures.
- `DEV_VERIFY_SKIP_CORPUS=1 ILANG_BROWSER_PORT=4184 scripts/dev-verify js`
  passes: **117/117 Node checks**, **60/60 browser checks** across Chromium,
  Firefox and WebKit. This includes all fetch cases at O0/O2, compiled host
  calls after ordinary errors, R2 seeded replays and every runnable R2 law
  body at both optimization levels in each engine.
- Final std check and both full std runs pass: **2,171/2,171 at O0 and O2**.
- Filtered release spec gates pass at both i levels: **9/ 149/149** and
  **11/ 48/48**. Native HTTP integration passes **11/11**.
- New note examples parse as complete module bindings. The two provider
  fixture manifests are now identical; their TOML-only snippet registrations
  coalesce under that literal's new identity. Every registered diagnostic
  expectation is unchanged, and all changed JS files' registrations resolve.
- Strict workspace Clippy, including every target, passes with `-D warnings`.
  All requested gates are green; no reported JavaScript runtime issue remains.


## Final verification commands

All heavy commands use `/home/agent/Programming/i/.rd/cache/heavy`, one at a
time. Cargo commands additionally use `direnv exec /home/agent/Programming/i`,
with `CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0` for development builds.
Logs are `/tmp/e4-jsfix-*.log`; investigation modules and WAT are in the ignored
`target/e4-js-debug/` directory.

| Command | Result |
|---|---|
| `cargo build --release -p ilang-cli --locked` | Both release builds pass: after E6 merge and after lowering fix |
| `i build --target js --opt 0 --wat` for compiled, fetch, and pre-E4-std fetch reproductions | All builds pass; baseline probes reproduce both traps |
| `i build --project tests/browser/fixtures/compiled --target host legacy --out tests/browser/fixtures/compiled.wasm` | Pass |
| `node --test bindings/javascript/test/compiled.test.mjs` | 8/8 pass, including reuse after ordinary failure |
| `i build --target js --opt 0 --wat` and `--opt 2 --wat` for corrected fetch reproduction; `node target/e4-js-debug/probe.mjs fetch <artifact>` | Both stream probes pass |
| `wasm-tools print <artifact> -o <wat>` on pre-port, baseline and corrected artifacts | Pass; signatures unchanged, retained Client delta visible |
| `I=$PWD/target/release/i bash scripts/build-abi-fixture.sh` | Pass, including conformance generation and Wasm validation |
| `DEV_VERIFY_SKIP_CORPUS=1 ILANG_BROWSER_PORT=4184 scripts/dev-verify js` | Final run: 117/117 Node, 60/60 browser checks; earlier runs exposed the diagnosed platform fixtures (108/117, then 114/117) |
| `node --test bindings/javascript/test/providers.test.mjs bindings/javascript/test/io.test.mjs` | 9/9 pass after those fixture fixes |
| `scripts/test-generation --mode short --binary ./target/release/i --output target/e4-js-debug/generation-short.json` | 32 cases, 64 permutations, 32 annotation removals; no failures |
| `i check --project std` | Pass |
| `i test --project std --opt 0` | 2171/2171 pass |
| `i test --project std --opt 2` | 2171/2171 pass |
| `taskset -c 0,1 cargo test --release -p ilang-cli --test spec --locked -- 9/` | 149/149 pass at both i optimization levels |
| `taskset -c 0,1 cargo test --release -p ilang-cli --test spec --locked -- 11/` | 48/48 pass at both i optimization levels |
| `cargo test --release -p ilang-cli --test http --locked -- --test-threads=1` | 11/11 pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass |
| `i fmt --check bench std lib examples` and the changed browser fixture sources | Pass; the process example's long shell argument was formatted before the final check |
| `cargo fmt --all -- --check`; `rustfmt --check --edition 2024 crates/ilang-wasm/src/lower.rs` | Pass |
| `node --check` on changed JavaScript tests; `python3 -m py_compile scripts/qa/generate.py`; `git diff --check` | Pass |
| Python audits of 75 std contracts/test retention, 720 old/new generation models, five normalized binding plans, and changed snippet registrations | Pass; no diagnostic expectation changed |
| `i fmt` on generated source samples; `i fmt` and `i fmt --check` on extracted note examples | Pass |

The coordinator-owned full workspace test suite and unfiltered review corpus
were not run by this lane. The full snippet gate was also not run; the focused
snippet audit covers all sources and registrations changed here. The general D16
compiler gap is unchanged; this fetch adapter uses the retained-frame pattern
required by runtime §6.3. No test is skipped, removed, or weakened.


## 2026-11-14 owner-ruling follow-up: external Fail needs

Syntax-three §2.10.12 says the function type of an `@external` declaration
“may have no need, or exactly `<- (Fail(E))` with a host-passable `E`”, and
“The glue reads `Fail` from `$env` and calls its `raise` with the host’s error.”
Spec §10.3 restricts errors to the six scalar ABI kinds, excluding opaque
references. The resolver still rejected every external need. It now accepts
exactly this case and retains all other need/error-shape rejections. The old
negative f4 fixture uses Assert instead; a new positive pins Fail(String),
its actual HostFn error list, and four independently invalid alternatives.
The external project golden now reports two complete invalid-need errors;
the obsolete separate-channel diagnostics are removed under E3 §6.1.

The external import’s Wasm signature already carries error slots, but its ABI
JSON omitted them. The new native execution regression exposed this exact
mismatch (six expected result slots versus three declared ones). Emit the
error list and use the existing typed reply path in both hosts. ABI5 permits
these errors; ABI3/4 retain their prior error-free external contract and the
existing invalid-metadata tests. Rust provides `fallible_external`; JS uses
`GuestError`, as for host services. Ordinary host failures still stop.

The new `external-fail` fixture keeps the existing compiled hypot program
unchanged and exercises success, the exact error value, and another success
on the same instance at O0/O2. The generation script builds its artifacts;
no binary or bindings file is edited by hand. Native O0/O2 execution and the
three checker-project tests pass. Node/browser execution and generated
artifacts are pending at this source commit; this is not a landable claim.

## 2026-11-14 nested-run re-entry follow-up

The current Node gate passes 122/123 tests: fetch streaming, compiled hypot,
and synchronous external Fail replies pass. The remaining law is
`9.9/typed_final_fresh_scope`, which traps in O0's `$s1` (function 2194), the
segment of run that matches its internal `Result(Result(A, Never), ScheduleError)`.
The WAT reaches the otherwise-impossible unmatched branch. The run row
snapshot keeps the Fail slot (1) and excludes only routing slots 2–6, but its
new scheduled callback tail-calls `body() <- env`; no frame retains that
provide across suspension.

This is the documented D16 tail-environment gap. Runtime design §6.3 requires:
“Use a non-tail call after those environment statements, through a surviving
identity frame when necessary”, explicitly including “nested run wrappers”.
Spec §9.4 rule 3: “The paused part keeps the services provided inside it”.
The patch uses `retain(body() <- env)`, retaining the actual caller row while
leaving the Scheduler-free public contract intact. A new §9.9 fixture reads a
provided Sample value before and after three yields, then asserts the exact
raised error through fail.catch. Every existing cleanup/cancellation/output
assertion remains. This patch is awaiting its native and Node O0/O2 reruns;
no final is installed or run on a stop.
