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
`npm run check:rendering` verifies example and titled transcript blocks in the
built site, including that execution metadata stays out of the displayed code.
After building, `npm run check:placeholders` runs its negative-fixture tests
and scans every HTML file in `dist`, including generated library pages.
It rejects `to be written` without regard to case or intervening HTML whitespace,
and the whole words `TODO` and `TBD` (in that case). It checks body text, including
code, `noscript`, and text in elements marked `hidden`. It ignores HTML comments,
attribute values, and the contents of `head`, `script`, `style`, and `template`.
Inline element boundaries preserve words; block boundaries act as whitespace.
This is a text check, not a CSS visibility calculation. Neither generated pages
nor any particular route is excluded. Both docs workflows run it after building.
Generation also runs before `npm run dev`. The standard's location is defined
by `SPEC_PATH` in `scripts/paths.py` for the planned relocation.
Generated pages are not edited by hand.

The home page is rendered from `content/home/index.mdx`, a separate Fumadocs
collection that uses the same code blocks and themes as the docs. Both parser
checks and the example runner read it. Its shared project files are
read by the runner without repeating them in the rendered page; their equality
with the hero is checked by `scripts/run-examples-test`.
Set `I_SNIPPET_PARSER` to an existing `parse-snippets` driver from the same tree
to run the parser checks without a Cargo build.

Example fences in authored pages use these info strings:

An expected-output `text` fence may carry a quoted `title="…"` for display;
it must have no other metadata. Hidden source fences still execute. The home
page uses a hidden import module to check the full inferred types of its shared
port reader, and a hidden caller to compare the custom `if` expansion and run it.
The displayed reader and macro definitions must match their shared files.

- `i run`: complete `src/main.i`; the next fence is `text`, with exact stdout
  (one trailing newline is tolerated). Runs at `--opt 0` and `--opt 2`.
  Add `hosts=native,wasi,javascript` to compare that same stdout on all three
  hosts at both levels: `i run`, a WASI build run by Node's WASI runner, and a
  host build run by the JavaScript SDK with `standardServices: true`. Node must
  be on PATH. Each example's existing deadline covers all six runs and builds;
  an unavailable host or a failed build fails the example, rather than skipping it.
- `i test`: complete `test/<page>_test.i`; every test must pass at both levels.
  Add `stdout=exact` and an immediately following `text` fence to compare its
  transcript as well as its exit status at both levels.
- `i test-fails`: `i test` must exit nonzero at both levels, with stdout exactly
  equal to the following `text` fence. A reproduction `sh` fence may precede it.
- `i check`: complete `src/main.i` that checks without diagnostics.
- `i types`: complete module checked with `i check --types --offline`; the next
  `text` fence must match its entire stdout exactly, with no diagnostics. This
  runs once because type checking has no optimization level.
- `i format`: unformatted complete module; `i fmt` formats the primary file and
  the next `text` fence must match the resulting file exactly. Runs once.
- `i expand`: complete module expanded with `i expand --offline --to i <file>`;
  the next `text` fence matches the entire expansion exactly, including its
  module comment and final blank line, with no diagnostics. Runs once.
- `i error E0123 E0456`: complete module whose check reports exactly that code set.
  Add `stderr=exact` and a following `text` fence to compare its entire diagnostic
  transcript as well.
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
`project=examples/port` loads a UTF-8 project template relative to the page.
The directory must be below the page directory, without parent traversal or
symlinks. The runner loads its files except root `.git`, `target`, `build` and
`generated` directories; preceding `file=` fences override those files, and the
primary example overrides the template's primary file. The runner still sets
the manifest name, local `std` path and test collection as usual. Shared files
are compiler inputs, not additional rendered code blocks.
`env.PORT=8080` supplies a deterministic environment input to an example at both
levels. It affects that example's compiler/program subprocesses only; the source
still uses the ordinary process environment service.
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

`npm run check:links` runs the per-page, font and carousel checks before link
validation. Both workflows run this command after building; failures propagate.

The per-page check (`npm run check:spec-links`) counts section marks and links
targeting `/docs/standard` or a file named `spec.md`. Home allows neither, and
also rejects every in-page `href="#…"`. Tour and getting-started allow one
combined reference; guides allow two. Library pages allow no specification
links. Contributor and standard pages are exempt from citation limits.

The same check prints prose words and complete example counts for every built
page. Prose is the authored page content (`#nd-home-layout` / `.home-page` or `#nd-page`), including
headings and inline code, outside code figures/blocks, navigation and controls.
Comments, head/script/style/template contents, `hidden`, `aria-hidden` and
`sr-only` text do not contribute words. Citation counts still include code and
hidden body text. Examples come from complete `run`, `test`, `test-fails`,
`check`, `error`, `types`, `format` and `expand` fences in the matching source page;
files, bare fragments, shown files and generated spec fragments are excluded.
Each example counts once, regardless of its optimization levels.

Home has a hard limit of 220 prose words. The following budgets are warnings
until enabled: tour ≤450 words and ≥6 examples; getting-started ≤300 words;
guides and contributing ≤500 words; library and standard **index pages** ≤250
words and 3–5 examples. Generated reference pages have no prose/example budget.
Use `--enforce-area tour` on `scripts/check-spec-links.py` to turn an area's
warnings into failures; repeat the option for more areas. The coordinator can
append these options to `check:spec-links` in package.json as the rewrites land.
Reviewer claim-to-rule tables belong in reports, not page text.

The home carousel is a native horizontal scroll-snap list. Without JavaScript,
all slides remain in the document and can be scrolled or focused with Tab.
JavaScript reveals named previous/next buttons and a live position indicator,
adds Left/Right paging, and updates the position after native swipe/scroll.
Focused code scrollers keep their own arrow keys. There is no autoplay. Reduced
motion uses instant paging; page swaps disconnect listeners and observers.
`npm run check:carousel` tests these controls with DOM fixtures, without opening
a browser. The home page overrides only its MDX heading components to plain
headings, so they add no hash links.

For a deterministic screenshot of the second slide, wait for
`document.fonts.ready`, then use this without changing the URL:

```js
const track = document.querySelector('[data-carousel-track]');
const second = track.querySelector('[data-home-slide="2"]');
track.scrollTo({
  left: second.offsetLeft - track.firstElementChild.offsetLeft,
  behavior: 'instant',
});
```

Both native scrolling and the enhancement use those same offsets. The card can
also be focused directly or brought into view with `scrollIntoView`.

Fonts are self-hosted from `@fontsource-variable/dm-sans` and
`@fontsource/dm-mono` (both 5.3.0). Only Latin WOFF2 assets are imported. DM Sans
uses normal/italic variable files for weights 400–700, replacing separate files
for the text, navigation and heading weights. DM Mono uses normal 400 and 500;
no unused 300 face, Latin-ext subset, serif face or legacy WOFF file is shipped.
All four faces use `font-display: swap`. Code has explicit four-space tab stops,
ligatures/contextual alternates disabled, and no synthetic bold/italic.

The sans fallback stack is `"DM Sans", system-ui, -apple-system,
BlinkMacSystemFont, "Segoe UI", "Noto Sans CJK SC", "PingFang SC",
"Microsoft YaHei", sans-serif`. The mono stack is `"DM Mono", ui-monospace,
"SFMono-Regular", Menlo, Consolas, "Liberation Mono", "Noto Sans Mono CJK SC",
"PingFang SC", "Microsoft YaHei", monospace`. Missing glyphs, including Chinese,
use these installed system fonts; no remote font host is contacted.

`npm run check:fonts` tests negative fixtures and checks every built font file
against the hashes of the four imported package assets. It rejects other font
bytes, non-DM web-font families, non-swap font faces, remote font URLs and remote
stylesheet imports/references. The asset list lives in `scripts/check-fonts.py`;
update it together with the CSS if the shipped faces deliberately change.
