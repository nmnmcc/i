# Tests through ordinary macros

The `i/test` library exports a collection anchor and the item macros `test`,
`suite`, `skip`, `only`, `fails`, and `each`. They use `Items` in
`meta.md §4.5`; the compiler does not recognize these names. Applications may
define and consume their own collections through the same mechanism.

```i
(test = i/test)

test.test("addition", () { assert(1 + 1 == 2) })
test.suite("arithmetic", () {
  ! Log = quiet_log
  test.test("zero", () { assert(0 + 0 == 0) })
  test.fails("expected failure", () { assert(False) })
})
```

The library can also export the macros directly, allowing the requested
`test("addition", () { ... })` spelling. The module-qualified examples use
ordinary module qualification; they do not make `test` into a special object.
`suite` takes a literal name and a function-shaped body containing environment
statements and nested item calls. It expands each nested declaration with a
copy of the enclosing environment statements before the test body. Nested
suites concatenate their names and environments from outermost to innermost.
The resulting bodies are separate functions, so each test gets its own scope
and resources. Names and bodies retain their original syntax scopes.

`test` expands to a hygienically private zero-argument function containing its
body and calls `Items.add` with the test collection, that function's name,
the display name and mode as String metadata, and the original call site.
The binding and the request are committed together only if expansion succeeds.
`skip`, `only`, and `fails` use the same expansion with mode metadata. `each`
accepts a literal list of `(name, value)` pairs and a function body, emits a
distinct target per pair, and includes the case name in the display name.
Expressions in the values remain ordinary i expressions in the generated
body; discovery does not evaluate them.

The test runner reads the checked collection manifest, orders it as specified
by §4.5, and selects targets before building provider plans. It filters names
as literal text, excludes skipped tests, and, when any `only` is present,
selects only those tests. A normal test succeeds when its entry completes
without an uncaught error or failed assertion. `fails` succeeds only when the
entry fails. Duplicate full names are errors at both source locations. The
manifest supplies the original call location and expansion provenance; a
generated binding name is not shown as the test name.

Migration replaces every `@test` function by a `test` call whose description
is the original function name, preserves the body and enclosing environment,
and imports the library macro in that module. Tests relying on a test function
as a separate binding retain an ordinary helper binding and call it inside the
test body. Once the migrated corpus passes, remove `@test` from the language.

ExUnit's `test` and `describe` macros register named functions in
`elixir/lib/ex_unit/lib/ex_unit/case.ex`; `@effect/vitest` supplies scoped and
layered fixtures in `effect/packages/vitest`. i's collection contract makes the
registration general, while i environment statements supply scoped services.
