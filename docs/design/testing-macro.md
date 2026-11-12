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
entries with that qualified collection identity, and compiles all selected roots into one artifact, then runs each in a fresh instance. Metadata supplies display names and modes, not type information.
`only` selects focused entries; `skip` is reported without execution.

`@test` has been removed. Only the configured collection supplies tests; there is
no attribute discovery or compatibility path.

Prior art: ExUnit's `case.ex` defines a function and registers its metadata;
Rust's `library/test` executes a collected descriptor list; Effect Vitest's
`internal/internal.ts` composes enclosing layers for nested suites; Julia's
`Test.jl` collects results under test sets.

## Migration

Run `scripts/test-macro-migrate <dir>...` on package directories or individual
`.i` files. Use `--embedded --standalone` to include Rust and JavaScript
literals, Markdown i fences, and editor corpus sections. Embedded temporary
package manifests must also configure the collection and depend on `std`. The tool uses parser spans, keeps the original body and comments,
and adds the collection and a `std` dependency where needed. It preserves a
function when its name is referenced or exported. Re-running it is idempotent.

The tool reports files needing manual work and exits non-zero for unsupported
attributes, conflicting imports or collection settings, parse errors in files
with tests, and source files without a package manifest (except standalone spec
fixtures). In particular, a fixture testing the validity of `@test` itself
cannot be converted without changing the subject of that fixture.

See `test-migrate-report.md` for repository migration counts and validation.
