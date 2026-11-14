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
Generation also runs before `npm run dev`. The standard's location is defined
by `SPEC_PATH` in `scripts/paths.py` for the planned relocation.
Generated pages are not edited by hand.

Example fences in authored pages use these info strings:

- `i run`: complete `src/main.i`; the next fence is `text`, with exact stdout
  (one trailing newline is tolerated). Runs at `--opt 0` and `--opt 2`.
- `i test`: complete `test/<page>_test.i`; every test must pass at both levels.
- `i check`: complete `src/main.i` that checks without diagnostics.
- `i error E0123 E0456`: complete module whose check reports exactly that code set.
- `i file=src/name.i`: an extra file for the next complete example on the page.
  Multiple files may precede it; paths are relative to the scratch project.
  Other files, such as `i.toml`, are allowed and are excluded from the i parser.
- Bare `i`: a fragment, checked only by the parser. Use it to discuss lines of
  a complete example on the same page.

Fumadocs displays filenames with `i file=src/name.i title="src/name.i"` or
`i title="src/name.i"`. The runner also accepts `i file="src/name.i"`.

Temporary projects use the page's filename as their package name, lowercased
with punctuation changed to underscores (`first-program.mdx` → `first_program`;
prefix `page_` if the name starts with a digit). Use that name for local imports.
The runner sets the manifest name, absolute local `std` paths, and the
`std/test.tests` collection. Tests use a `std` dev-dependency unless source
modules need a regular dependency. Commands use `--offline`; output must be
deterministic. Each example has a 60-second deadline across both levels.

```sh
I_BIN=/path/to/i python3 docs/scripts/run-examples.py --jobs 4 --page 'docs/tour/*.mdx'
I_BIN=/path/to/i python3 docs/scripts/run-examples-test
```

`--page` is repeatable and matches paths relative to `docs/content`, `docs`, or
the repository root. The default compiler is `target/release/i`; if absent, the
runner builds it offline through the shared heavy queue when present.
`--no-placeholders` rejects `to be written` in authored content; it is off by
default until the text lands. Generated directories are excluded from both checks.
