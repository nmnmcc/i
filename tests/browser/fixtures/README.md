# Host ABI fixtures

These modules are written by hand to test the Rust host (`crates/ilang-host`)
and the JavaScript host (`bindings/javascript`) against the binary interface
version 3 of `docs/spec.md` (Appendix B). They are not compiler output, except
`compiled.wasm`.

Regenerate the checked-in binaries with `bash scripts/build-abi-fixture.sh` from
the Devenv shell, which has wasm-tools (`WASM_TOOLS` can point at another
executable). The script builds `host-abi`, `required-features`,
`custom-imports`, `one-thread`, `no-continue`, `wrong-types` and
`start-request`, validates them, checks that
every file that names the ABI version says what the compiler writes, and writes
the feature probe into `bindings/javascript/ilang.mjs`.

## host-abi.wat

It has its own `ilang.abi` section. Its allocator only counts live buffers:
`live_allocations` and `live_bytes` let tests check every free of B.3. Size 0
gives address 0.

The sparse request table uses linear memory: 1024 state words at 4096 and
1024 expected-argument words at 8192. Allocations start at 12288, after both
tables, so a result buffer cannot overwrite a request's expected answer.

- `ilang_start` prints `start` and a line feed through the service, so tests see
  that it runs once and may call imports.
- Imports: `ilang:host/v1` `panic`, `reference_id` and `cancel`; the services
  `app:console/v1` `print(String) -> Nil`, `app:storage/v1`
  `read(String) -> String ? String`, the async `app:clock/v1`
  `later(Float) -> Float ? String`, whose import takes the request id first
  (B.4); the external `app:math/v1` `hypot(Float, Float) -> Float`.
- Round trips of every type: `string_roundtrip`, `int_roundtrip`,
  `bit_array_roundtrip`, `float_roundtrip`, `bool_roundtrip`, `nil_roundtrip`,
  `opaque_roundtrip`, and `same_reference` through `reference_id`.
- `choose(Float) -> String ? Nil | Bool | Int | Float | String | BitArray`:
  selector 0 gives `"ok"`, 1 to 6 an error of each member, 7 a tag that is too
  big.
- `read_through`, `hypot` and `print` forward to the imports.
- `stop(String)` gives the text to `panic` and traps.
- `exceptions` throws and catches a Wasm exception inside the module.
- `bad_bool`, `bad_utf8` and `bad_tag` give values that are not correct.
- `app/async.later` is an async export: it waits for `later` twice through
  `ilang_resume:app:clock/v1:later` and gives its results through
  `ilang_result:app/async.later`. It takes ids from a counter, and the resume
  takes the id first and traps (a bare `unreachable`, no stop message) when it
  is not the id of the outstanding request.
- `app/deep(Float) -> Float` recurses n deep, not in tail position, and gives
  n. The async `app/async.deep(Float) -> Float` waits for `later(1.0)` and does
  the same recursion inside `ilang_resume:app:clock/v1:later`, so tests can
  compare the stack depth of a sync call and of a resume.
- The exports below keep many requests outstanding (B.4, B.5), which a
  module built by the compiler never does yet. They share these conventions:
  - Requests have the ids `k*37+5` and the arguments `x = k+1` for
    k = 0, 1, …, unless a line below says otherwise. The test hosts answer x
    with `2x`, and hold every request whose argument is 0: they never answer
    it.
  - Each answer is checked: an id that is not outstanding panics with
    `fixture: bad id`, an id that was cancelled with `fixture: cancelled
    request answered`, and a value that is not `2x` for the current request
    of that id, or an error, with `fixture: wrong answer`. The module folds
    the arguments in answer order into `h = (h * 31 + x) mod (2^31 - 1)`,
    which `ilang_result:…` gives as a Float, so a test can compare it with
    the order in which its host answered.
  - The one `ilang_continue` export serves `app/async.yielding` and
    `app/async.continue_io`, by a mode global that each sets. Another global
    keeps the last status that any export gave; `ilang_continue` traps (a bare
    `unreachable`) when it is not 2 (B.5).
  - `ilang_alloc` starts the request 77 (for `app/async.nested`) or cancels
    the request 5 (for `app/async.nested_cancel`) when one of them has set
    its flag.
- `app/async.many(n)` starts n requests. After each of the first n answers it
  starts one more with the freed id and the next argument, while the others
  are still outstanding, and ends after 2n answers.
- `app/async.fan(n)` starts n requests and ends after n answers.
- `app/async.cancel(n)` starts n requests. On each answer it cancels the
  outstanding request with the lowest id, and ends when none is outstanding.
- `app/async.cancel_reuse(n)` starts n requests. On each answer it cancels the
  outstanding request with the lowest id and at once starts a new one with
  that id and the next argument, until 2n have started. A stale answer of the
  cancelled request gives `fixture: wrong answer`.
- `app/async.yielding(m)` starts one request (id 5, argument 1) and gives
  status 2. `ilang_continue` gives 2 until the answer has come, then 0; the
  result is the number of its calls. m calls with no answer panic with
  `fixture: starved`.
- `app/async.continue_io(n)` gives status 2 with nothing outstanding. Each
  call of `ilang_continue` starts one request, k = 0, 1, …; the first has the
  argument 0, which test hosts hold, and the second call cancels it. It gives
  2 until n have started, then 1, and ends when every request that was not
  cancelled is answered.
- `app/async.leftover` starts 3 requests and ends with status 0 after the
  first answer (B.5: the host drops the others).
- `app/async.stop_mid` starts 3 requests and stops with `fixture: stop` on the
  first answer.
- `app/async.stuck` gives status 1 with nothing outstanding;
  `app/async.bad_status` gives status 7.
- `app/async.dup` starts two requests with the id 7; `app/async.negative`
  starts one with the id -1.
- `app/async.cancel_bad` cancels the request 12345, which is not outstanding.
- `app/async.nested` and `app/async.nested_cancel` call `read`, whose String
  answer makes the host call `ilang_alloc` during the import, which then
  starts or cancels a request.
- `app/async.result_starts` and `app/async.result_cancels` start one request
  (id 5, argument 1) and end after its answer; their `ilang_result:…` starts
  the request 6 or cancels the request 0.
- `app/sync.starts` and `app/sync.cancels` are sync exports that start the
  request 3 or cancel the request 0.
- The entry `app/main.main` prints `fixture main`. The tests are
  `app/test.passing`, `app/test.failing` (a stop with a message),
  `app/test.trapping` (a trap without one), `app/test.fresh` twice (fails when
  run twice in one instance), and the async `app/async_test.waits`.
- `feature_probe`, `tail_count` and `gc_sum` use typed function references, tail
  calls and GC structures.

## custom-imports.wat

It has no metadata; tests give it (see `tests/browser/fixture-abi.mjs` and
`crates/ilang-host/src/tests.rs`). It imports the externals `echo_string`,
`echo_int`, `echo_bits`, `echo_ref`, `echo_bool`, `echo_float` and `echo_nil`
from module `custom`, and the service `custom:fallible/v1`
`fallible(Float) -> String ? Nil | Bool | Int | Float | String | BitArray`.
Each export forwards to one import. The module makes and frees the parameter
buffers of the imports and gives the result buffers of the imports on to the
host as its own results.

## one-thread.wat

It has its own `ilang.abi` section and imports `app:console/v1`
`print(String) -> Nil` and the async `app:clock/v1`
`later(Float) -> Float ? String`. Its async entry `app/main.main` and its async
test `app/test.waits` each print `begin`, then wait for `later` three times in a
row and print `answer` after each answer: 4 prints and 3 starts of `later`,
with ids from a counter. The Rust host checks with it that a whole run uses one
thread.

## no-continue.wat

It has its own `ilang.abi` section and imports the async `app:clock/v1`
`later(Float) -> Float ? String`. The async `app/async.runnable` gives status 2,
but the module has no `ilang_continue`. The async `app/async.no_resume` starts
the request 0 of `later`, but the module has no
`ilang_resume:app:clock/v1:later`. Both are faults of the module (B.5). The
sync `app/sync.nil` counts its calls in `calls`, so tests can check that the
host refuses it afterwards.

## required-features.wat

A dependency-free probe with a GC structure, a typed tail call through
`return_call_ref`, and `try_table` with `catch_ref` and `throw_ref`. Its `probe`
export gives 42. The SDK validates it to detect the required features.

## compiled.wasm

Built by the compiler (`i build`) from the package in `compiled/`, so
that the hosts run real compiler output. The script above builds it again. It
uses the host functions of `hostImports` in `tests/browser/fixture-abi.mjs`, and
an async `app:clock/v1` `tick(Int) -> Int` (`compiledImports`):

- `app/app.greet(String) -> Int` prints through the sync service `print`.
- `app/app.lookup(String) -> String ? String` reads through `read`.
- `app/app.distance(Float, Float) -> Float` calls the external `hypot`.
- `app/app.wait_sum(Float, Float) -> Float ? String` is async: it waits for
  `later` two times.
- The async entry `app/app.main` waits for `tick` and `later`, and prints.

`crates/ilang-cli/tests/commands.rs` checks that its metadata is what the
compiler gives now.

## wrong-types.wat

Like `no-continue.wat`, each async export breaks a rule of B.5, here with an
export of the wrong type, which is a fault like a missing one:
`app/async.runnable` gives status 2, and `ilang_continue` gives no status;
`app/async.bad_resume` starts a request of `later` (id 0), and
`ilang_resume:app:clock/v1:later` has no request id first. `calls` counts the
calls of these two exports and of the sync `app/sync.nil`, which the host must
not make.

## start-request.wat

Its `ilang_start` starts a request of the async `later` (id 0), which B.4 does
not allow outside an async call: the instantiation fails, and the service does
not run.

Both `no-continue.wat` and `wrong-types.wat` also check when the required
exports apply (B.5). `app/async.runnable_ready` starts request 0 of `ready`
and returns status 2. Its correctly typed resume would return 0, so ready
work must not hide an invalid `ilang_continue`. The `no_resume_done` and
`no_resume_cancel` exports in `no-continue.wat`, and `bad_resume_done` and
`bad_resume_cancel` in `wrong-types.wat`, start request 0 of `later`, then
return 0 or cancel it. The import must fault before either action can hide
an invalid resume export. `calls` also counts resume and result extraction
for these cases, so it stays 0 when the host catches the fault in time.
