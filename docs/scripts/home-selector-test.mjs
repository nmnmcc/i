import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';

const page = readFileSync(new URL('../content/home/index.mdx', import.meta.url), 'utf8');
const css = readFileSync(new URL('../src/styles/home.css', import.meta.url), 'utf8');
const keys = ['simple', 'safe', 'testable', 'precise', 'extensible', 'fast'];
// Escape delimiters so the snippet inventory does not read regex syntax as source literals.
const panels = [...page.matchAll(/<section className=\x22example-panel\x22 id=\x22panel-([^\x22]+)\x22[^>]*>([\s\S]*?)<\/section>/g)];

test('six native radios select six labelled regions in reading order', () => {
  assert.deepEqual(panels.map((match) => match[1]), keys);
  const inputs = [...page.matchAll(/<input\b([^>]+)\/>/g)].map((match) => match[1]);
  assert.equal(inputs.length, 6);
  inputs.forEach((input, index) => {
    const key = keys[index];
    for (const attribute of ['type="radio"', 'name="home-example"', `id="choice-${key}"`, `aria-controls="panel-${key}"`, `aria-labelledby="title-${key}"`, `aria-describedby="description-${key}"`]) assert.ok(input.includes(attribute));
    assert.equal(/defaultChecked=\{true\}/.test(input), index === 0);
    assert.ok(css.includes(`:has(#choice-${key}:checked) #panel-${key}`));
    assert.ok(page.includes(`<label className="home-choice" htmlFor="choice-${key}">`));
  });
  assert.doesNotMatch(page, /role=\x22tab|aria-selected=/);
  assert.match(page, /<fieldset[^>]*aria-label=\x22Choose an example\x22/);
});

test('all displayed program sources fit fourteen lines and sixty-four columns', () => {
  for (const [, key, panel] of panels) {
    const visible = panel.replace(/<div hidden>[\s\S]*?<\/div>/g, '');
    const programs = [...visible.matchAll(/^\x60{3}(?:i|scheme)\b[^\n]*\n([\s\S]*?)^\x60{3}/gm)].map((match) => match[1]);
    assert.ok(programs.length > 0, key);
    const commands = [...visible.matchAll(/<pre className=\x22host-command\x22>\{([^\n]+)\}<\/pre>/g)].flatMap((match) => JSON.parse(match[1]).split('\n'));
    const lines = [...programs.flatMap((program) => program.trimEnd().split('\n')), ...commands];
    assert.ok(lines.length <= 14, `${key}: ${lines.length} source lines`);
    for (const line of lines) assert.ok(line.replaceAll('\t', '  ').length <= 64, `${key}: ${line}`);
  }
});

test('page removes the port hero, lone Enough, hashes and carousel', () => {
  assert.doesNotMatch(page, /\bPort\b|PORT=|examples\/port|hero-signature|home-carousel|href=[\x22\x27]#/);
  assert.doesNotMatch(page, />Enough\.<\//);
  assert.match(page, /href=\x22\/docs\/tour\x22/);
  assert.match(page, /href=\x22\/docs\/getting-started\/install\x22/);
  assert.doesNotMatch(readFileSync(new URL('../src/pages/index.astro', import.meta.url), 'utf8'), /<script>/);
});

test('ordinary macro use comes before expansion and the Lisp definition', () => {
  const macro = panels.find((match) => match[1] === 'extensible')[2];
  assert.ok(macro.indexOf('```i run') < macro.indexOf('```text title="i expand'));
  assert.ok(macro.indexOf('```text title="i expand') < macro.indexOf('```scheme'));
  assert.match(macro, /<summary>Lisp surface: src\/sugar\.is<\/summary>/);
  assert.ok(page.includes('Write your own <code>if</code> when you need one.'));
});

test('plain wording and clean inferred service types stay in the page', () => {
  assert.doesNotMatch(page, /\b(novel|unique|powerful|revolutionary|advanced|first|only|guarantee|set-theoretic|capability|intersection)\b|effect system|principal types|i\/request\.(?:Await|Owner)/i);
  assert.match(page, /remaining : \(Int\) <- \(Clock, Fail\(Late\)\) -> Int/);
});
