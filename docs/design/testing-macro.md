# Test macros

`std/test` supplies item macros. Import the names using an alias that does not
shadow `test`, for example `(testlib = (test, suite, skip, only, fails, each) = std/test)`.
The variant names are separate imports: §2.2 reserves `a.m(...)` for module
qualification, so `test.skip(...)` would mean a module named `test`.

`test("name", () { ... })` expands to a fresh private, zero-argument function
that calls the supplied body. The item expansion contributes that function to
`std/test.tests` through `Items.add`, with `name` and `mode` string metadata.
`skip`, `only` and `fails` generate the same kind of entry, with their respective
modes. Each function is a distinct selected root and runs in a fresh instance.
A failure or uncaught typed error belongs to that entry. An expected failure
passes only when execution fails; a successful body is a failure.

`suite("name", () { ... })` expands its contained calls to items, prefixing
names with `name / `. Environment statements are copied in order into each
following test body, including nested suites; later statements replace earlier
services according to the ordinary environment rules. `each("name", [("case",
value), ...], (value) { ... })` expands to one `test` per case, named
`name / case`. Names and case lists are written literals so discovery does not
execute application code. Suites and cases compose.

A package opts in with `[test] collection = "std/test.tests"` in `i.toml` and
a dependency on `std`. The runner reads the general collection manifest, selects
entries with that qualified collection identity, and compiles and runs each
selected root. Metadata supplies display names and modes, not type information.
`only` selects focused entries; `skip` is reported without execution. Legacy
`@test` entries remain available during migration.

The existing 3,015 `@test` functions can be migrated mechanically: add the
import and test collection configuration, replace each `@test name = () { body }`
with `test("name", () { body })`, preserve the body and comments, and then run
`i fmt` and `i test`. Scripts should use the parser, rather than regular
expressions, to preserve multiline bodies and annotations. The old attribute is
removed only after all packages have migrated.

Prior art: ExUnit's `case.ex` defines a function and registers its metadata;
Rust's `library/test` executes a collected descriptor list; Effect Vitest's
`internal/internal.ts` composes enclosing layers for nested suites; Julia's
`Test.jl` collects results under test sets.

## Migration

Run `scripts/test-macro-migrate <dir>...` on package directories or individual
`.i` files. The tool uses parser spans, keeps the original body and comments,
and adds the collection and a `std` dependency where needed. It preserves a
function when its name is referenced or exported. Re-running it is idempotent.

The tool reports files needing manual work and exits non-zero for unsupported
attributes, conflicting imports or collection settings, parse errors in files
with tests, and source files without a package manifest (except standalone spec
fixtures). In particular, a fixture testing the validity of `@test` itself
cannot be converted without changing the subject of that fixture.

On a copy of `std`, 1,845 tests passed at `--opt 0` both before and after
migration. The copied spec corpus contains 239 migrated test functions; one
fixture retains its invalid `@main`/`@test` combination for manual review.
