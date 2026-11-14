#!/usr/bin/env node
/** Browser review gate. Uses the root package's existing Playwright dependency. */
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, writeFile, mkdir, mkdtemp } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { join, extname, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from '@playwright/test';

const root = fileURLToPath(new URL('../../', import.meta.url));
const dist = join(root, 'docs/dist');
const executablePath = process.env.CHROMIUM_BIN;
assert.ok(executablePath, 'Set CHROMIUM_BIN to an existing Chromium executable; this check never downloads a browser.');
const output = process.env.DOCS_LAYOUT_OUTPUT ?? await mkdtemp(join(tmpdir(), 'i-home-layout-'));
await mkdir(output, { recursive: true });
const keys = ['simple', 'safe', 'testable', 'precise', 'extensible', 'fast'];
const source = await readFile(join(root, 'docs/content/home/index.mdx'), 'utf8');
const sourceMetrics = Object.fromEntries([...source.matchAll(/<section className=\x22example-panel\x22 id=\x22panel-([^\x22]+)\x22[^>]*>([\s\S]*?)<\/section>/g)].map(([, key, body]) => {
  const visible = body.replace(/<div hidden>[\s\S]*?<\/div>/g, '');
  const programs = [...visible.matchAll(/^\x60{3}(?:i|scheme)\b[^\n]*\n([\s\S]*?)^\x60{3}/gm)].flatMap((match) => match[1].trimEnd().split('\n'));
  const commands = [...visible.matchAll(/<pre className=\x22host-command\x22>\{([^\n]+)\}<\/pre>/g)].flatMap((match) => JSON.parse(match[1]).split('\n'));
  const lines = [...programs, ...commands];
  const widest = Math.max(...lines.map((line) => line.replaceAll('\t', '  ').length));
  assert.ok(lines.length <= 14 && widest <= 64, `${key}: ${lines.length} lines / ${widest} columns`);
  return [key, { lines: lines.length, widest }];
}));
assert.deepEqual(Object.keys(sourceMetrics), keys);
const proseWords = Number(execFileSync('python3', ['-c', '# Python prose measurement\nimport sys; from pathlib import Path; sys.path.insert(0, sys.argv[1]); from page_prose import prose_words; print(prose_words(Path(sys.argv[2]).read_text()))', join(root, 'docs/scripts'), join(dist, 'index.html')], { encoding: 'utf8' }).trim());
assert.ok(proseWords <= 120, `${proseWords} prose words exceeds 120`);
const mime = { '.html': 'text/html', '.js': 'application/javascript', '.css': 'text/css', '.woff2': 'font/woff2', '.svg': 'image/svg+xml', '.webp': 'image/webp', '.json': 'application/json' };
const server = createServer(async (req, res) => {
  try {
    let file = resolve(dist, '.' + decodeURIComponent(new URL(req.url, 'http://localhost').pathname));
    assert.ok(file === dist || file.startsWith(dist + sep));
    if (!extname(file)) file = join(file, 'index.html');
    const body = await readFile(file);
    res.writeHead(200, { 'content-type': mime[extname(file)] ?? 'application/octet-stream' });
    res.end(body);
  } catch { res.writeHead(404); res.end('Not found'); }
});
await new Promise((done) => server.listen(0, '127.0.0.1', done));
let browser;
const measurements = [];
const pageErrors = [];
try {
  browser = await chromium.launch({ executablePath, headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage', '--force-prefers-reduced-motion'] });
  for (const [width, height] of [[1440, 900], [1280, 800], [1920, 1080], [390, 844]]) {
    for (const [theme, javaScriptEnabled] of [['light', true], ['dark', true], ['light', false]]) {
      const context = await browser.newContext({ viewport: { width, height }, colorScheme: theme, javaScriptEnabled, reducedMotion: 'reduce' });
      const page = await context.newPage();
      page.on('pageerror', (error) => pageErrors.push(String(error)));
      await page.goto(`http://127.0.0.1:${server.address().port}/`, { waitUntil: 'networkidle' });
      await page.evaluate(() => document.fonts.ready);
      const prefix = `${width}x${height}-${theme}-${javaScriptEnabled ? 'js' : 'no-js'}`;
      assert.ok(await page.locator('#choice-simple').isChecked());
      assert.equal(await page.locator('.example-panel:visible').count(), 1);
      let baseline;
      for (const key of keys) {
        await page.locator(`label[for="choice-${key}"]`).click();
        const panel = page.locator(`#panel-${key}`);
        await panel.waitFor({ state: 'visible' });
        assert.equal(await page.locator('.example-panel:visible').count(), 1);
        const data = await panel.evaluate((panel) => {
          const rect = (el) => { const r = el.getBoundingClientRect(); return { top: r.top + scrollY, bottom: r.bottom + scrollY, height: r.height }; };
          const blocks = [...panel.querySelectorAll('figure')].filter((figure) => !figure.closest('[hidden]') && !figure.closest('details:not([open])')).map((figure) => {
            const pre = figure.querySelector('pre'), region = figure.querySelector('[role="region"]');
            const caption = figure.querySelector('figcaption');
            return { title: caption?.textContent, captionFits: !caption || caption.scrollWidth <= caption.clientWidth + 1, output: !!figure.querySelector('code.language-text'), scrollWidth: region.scrollWidth, clientWidth: region.clientWidth, whiteSpace: getComputedStyle(pre).whiteSpace, overflowX: getComputedStyle(region).overflowX, fontSize: getComputedStyle(pre).fontSize, tabSize: getComputedStyle(pre).tabSize, maxHeight: getComputedStyle(region).maxHeight };
          });
          const intro = rect(document.querySelector('.home-intro')), display = rect(document.querySelector('.home-display'));
          return { scrollHeight: document.documentElement.scrollHeight, scrollWidth: document.documentElement.scrollWidth, stage: rect(document.querySelector('.example-stage')), actions: rect(document.querySelector('.home-actions')), panel: rect(panel), contentTop: Math.min(intro.top, display.top), contentBottom: Math.max(intro.bottom, display.bottom), blocks };
        });
        const label = `${prefix} ${key}`;
        assert.ok(data.scrollHeight <= (width > 800 ? height : height * 1.5), `${label}: scrollHeight ${data.scrollHeight}`);
        assert.ok(data.scrollWidth <= width, `${label}: horizontal page overflow`);
        assert.ok(data.panel.top >= data.stage.top - 1 && data.panel.bottom <= data.stage.bottom + 1, `${label}: example exceeds its fixed stage`);
        if (width > 800) {
          assert.ok(data.contentBottom <= height, `${label}: content is below viewport`);
          assert.ok(data.contentTop <= height * .25 && height - data.contentBottom <= height * .25, `${label}: oversized blank band`);
        }
        for (const block of data.blocks) {
          assert.ok(block.captionFits, `${label}: caption clipped ${block.title}`);
          if (block.output) {
            assert.equal(block.whiteSpace, 'pre-wrap', label);
            assert.equal(block.overflowX, 'hidden', label);
            assert.ok(block.scrollWidth <= block.clientWidth + 1, `${label}: output clipped ${block.title}`);
            assert.equal(block.maxHeight, 'none', label);
          } else if (width > 800) assert.ok(block.scrollWidth <= block.clientWidth + 1, `${label}: source clipped ${block.title} (${block.scrollWidth} > ${block.clientWidth})`);
          else { assert.equal(block.fontSize, '12px', label); assert.equal(block.tabSize, '2', label); }
        }
        if (baseline) {
          assert.equal(data.scrollHeight, baseline.scrollHeight, `${label}: page height changed`);
          assert.equal(data.stage.top, baseline.stage.top, `${label}: stage moved`);
          assert.equal(data.actions.top, baseline.actions.top, `${label}: actions moved`);
        } else baseline = data;
        await page.evaluate(() => window.scrollTo(0, 0));
        await page.screenshot({ path: join(output, `${prefix}-${key}.png`), fullPage: width < 801 });
        measurements.push({ viewport: [width, height], theme, javaScriptEnabled, key, ...sourceMetrics[key], ...data });
        console.log(`${label}: ${data.scrollHeight}px; ${sourceMetrics[key].lines} lines/${sourceMetrics[key].widest} columns; source fits; output wraps`);
      }
      // Native radio keyboard behavior works even when page scripting is disabled.
      await page.locator('#choice-simple').focus();
      await page.keyboard.press('ArrowRight');
      assert.ok(await page.locator('#choice-safe').isChecked());
      assert.ok(await page.locator('#panel-safe').isVisible());
      await page.keyboard.press('ArrowLeft');
      assert.ok(await page.locator('#choice-simple').isChecked());
      await page.keyboard.press('Tab');
      assert.equal(await page.locator(':focus').evaluate((el) => el.closest('.example-panel')?.id), 'panel-simple');
      // The small optional definition must fit too when opened.
      await page.locator('label[for="choice-extensible"]').click();
      await page.locator('.macro-definition summary').click();
      const expanded = await page.locator('#panel-extensible').evaluate((el) => ({ bottom: el.getBoundingClientRect().bottom, stage: el.parentElement.getBoundingClientRect().bottom, height: document.documentElement.scrollHeight }));
      assert.ok(expanded.bottom <= expanded.stage + 1, `${prefix}: open macro definition exceeds stage`);
      assert.ok(expanded.height <= (width > 800 ? height : height * 1.5));
      if (width > 800) {
        const definition = await page.locator('.macro-definition [role="region"]').evaluate((el) => ({ width: el.clientWidth, scroll: el.scrollWidth }));
        assert.ok(definition.scroll <= definition.width + 1, `${prefix}: macro definition source is clipped`);
      }
      await page.evaluate(() => window.scrollTo(0, 0));
      await page.screenshot({ path: join(output, `${prefix}-definition.png`), fullPage: width < 801 });
      await context.close();
    }
  }
  assert.deepEqual(pageErrors, []);
  await writeFile(join(output, 'measurements.json'), JSON.stringify({ proseWords, sourceMetrics, measurements }, null, 2));
  console.log(`${measurements.length} layouts passed; ${proseWords} prose words; screenshots: ${output}`);
} finally {
  await browser?.close();
  await new Promise((done) => server.close(done));
}
