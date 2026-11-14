# Documentation site

Use Node 22.23.4 or later. In the repository development shell:

```sh
cd docs
npm ci
npm run dev
```

The development server prints its local URL. `npm run build` generates the
standard and library reference before building the static site in `dist/`.
`npm run check:links` checks the built local links; `npm run check:examples`
checks fenced i examples with the repository's parser and snippet registry,
then executes complete examples against the tree's compiler.
`npm run check:rendering` verifies the built i blocks' titles and highlighted
source, including that execution metadata stays out of the displayed code.
Generation also runs before `npm run dev`. The standard's location is defined
by `SPEC_PATH` in `scripts/paths.py` for the planned relocation.
Generated pages are not edited by hand.

Example fences in authored pages use these info strings:

- `i run`: complete `src/main.i`; the next fence is `text`, with exact stdout
  (one trailing newline is tolerated). Runs at `--opt 0` and `--opt 2`.
- `i test`: complete `test/<page>_test.i`; every test must pass at both levels.
- `i test-fails`: `i test` must exit nonzero at both levels, with stdout exactly
  equal to the following `text` fence. A reproduction `sh` fence may precede it.
- `i check`: complete `src/main.i` that checks without diagnostics.
- `i error E0123 E0456`: complete module whose check reports exactly that code set.
- `i shown file=src/main.i`: a complete file checked with `i check`, without
  running it. Includes earlier `file=` blocks on the same page, including those
  already used by an earlier example, so a hand-run entry can reuse its modules.
- `i shown-unchecked file=src/main.i`: parse a complete source with `i fmt`,
  without type checking or running it. Reserve this for a project that cannot
  check alone, such as a client of a second package; list it in the report.
- `i file=src/name.i`: an extra file for the next complete example on the page.
  Multiple files may precede it; paths are relative to the scratch project.
- `scheme file=src/macros.is title="src/macros.is"`: a supporting Lisp-surface
  module. The site highlights it as Scheme; the snippet checks use the i
  parser driver's `--lisp` mode. The runner includes it in the compiled project.
- `toml file=i.toml title="i.toml"`: a supporting manifest. Use
  `text file=data.txt title="data.txt"` for other data. These files are written
  into the project and are excluded from source parsing. Non-source files must
  not use language `i`, which is reserved for `.i` modules.
- Bare `i`: a fragment, checked only by the parser. Use it to discuss lines of
  a complete example on the same page.

Use quoted titles consistently: `i run package=hello title="src/main.i"` and
`i file=src/name.i title="src/name.i"`. Fumadocs displays the title and highlights
the block as i; the other words are metadata. A title also names an example's
source file: `i test-fails package=hello title="test/failure_test.i"` keeps the
test module name in its transcript. Paths must be relative to the project;
primary sources live under `src/` or `test/`. The runner also accepts the older
`i file="src/name.i"` and `i title="src/name.i"` supporting-file spellings for
`.i` modules; other languages require an explicit `file=` attribute.

Temporary projects use the page's filename as their package name, lowercased
with punctuation changed to underscores (`first-program.mdx` → `first_program`;
prefix `page_` if the name starts with a digit). `package=<name>` overrides that
default on run, test, test-fails, check, error, and shown fences. The runner never
infers the package name from imports. Use the declared name for local imports.
The runner sets the manifest name, absolute local `std` paths, and the
`std/test.tests` collection. Tests use a `std` dev-dependency unless source
modules need a regular dependency. Commands use `--offline`; output must be
deterministic. Each example has a 60-second deadline across both levels.

```sh
I_BIN=/path/to/i python3 docs/scripts/run-examples.py --jobs 4 --page 'docs/tour/*.mdx'
I_BIN=/path/to/i python3 docs/scripts/run-examples-test
I_BIN=/path/to/i python3 docs/scripts/run-examples.py --content docs/scripts/run-examples-fixtures --page failing-run.mdx
```

`--page` is repeatable and matches paths relative to `docs/content`, `docs`, or
the repository root. The default compiler is `target/release/i`; if absent, the
runner builds it offline through the shared heavy queue when present.
`--no-placeholders` rejects `to be written` in authored content. It is enabled
by `npm run check:examples`; direct runner invocations opt in. Generated
directories are excluded from execution and placeholder checks.
The repository snippet inventory excludes `scripts/run-examples-fixtures/`
under `docs`: its deliberately valid and invalid inputs are checked by
`scripts/run-examples-test`, rather than treated as documentation.
