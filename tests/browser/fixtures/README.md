# Host ABI fixtures

These modules are written by hand to test the Rust host (`crates/ilang-host`)
and the JavaScript host (`bindings/javascript`) against the binary interface
version 2 of `docs/spec.md` (Appendix B). They are not compiler output, except
`compiled.wasm`.

Regenerate the checked-in binaries with `bash scripts/build-abi-fixture.sh` from
the Devenv shell, which has wasm-tools (`WASM_TOOLS` can point at another
executable). The script validates the modules and writes the feature probe into
`bindings/javascript/ilang.mjs`.

## host-abi.wat

It has its own `ilang.abi` section. Its allocator only counts live buffers:
`live_allocations` and `live_bytes` let tests check every free of B.3. Size 0
gives address 0.

- `ilang_start` prints `start` and a line feed through the service, so tests see
  that it runs once and may call imports.
- Imports: `ilang:host/v1` `panic` and `reference_id`; the services
  `app:console/v1` `print(String) -> Nil`, `app:storage/v1`
  `read(String) -> String ? String`, the async `app:clock/v1`
  `later(Float) -> Float ? String`; the external `app:math/v1`
  `hypot(Float, Float) -> Float`.
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
  `ilang_result:app/async.later`.
- `app/deep(Float) -> Float` recurses n deep, not in tail position, and gives
  n. The async `app/async.deep(Float) -> Float` waits for `later(1.0)` and does
  the same recursion inside `ilang_resume:app:clock/v1:later`, so tests can
  compare the stack depth of a sync call and of a resume.
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
row and print `answer` after each answer: 4 prints and 3 starts of `later`. The
Rust host checks with it that a whole run uses one thread.

## required-features.wat

A dependency-free probe with a GC structure, a typed tail call through
`return_call_ref`, and `try_table` with `catch_ref` and `throw_ref`. Its `probe`
export gives 42. The SDK validates it to detect the required features.

## compiled.wasm

Built by the compiler (`i build --opt 2`) from the package in `compiled/`, so
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
