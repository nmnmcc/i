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
checks fenced i examples with the repository's parser and snippet registry.
Generation also runs before `npm run dev`. The standard's location is defined
by `SPEC_PATH` in `scripts/paths.py` for the planned relocation.
Generated pages are not edited by hand.
