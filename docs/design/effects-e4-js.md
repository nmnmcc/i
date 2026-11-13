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
fail.catch(() { body() <- (http.fetch_client()) })
```

Its WAT ended in `return_call_ref`, so the provision had no surviving frame
and no delta to replay. This is the existing general tail-provide gap
explained by `lazy-reentry.md` D16 and §7a. The provider adapter now follows the
same retained-call pattern as the library adapters:

```i
fail.catch(() {
  value = body() <- (http.fetch_client())
  value
})
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
- Remaining std/spec/HTTP/Clippy gates: in progress.
