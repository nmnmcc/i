#!/usr/bin/env node
/** Browser review gate. Uses the root package's existing Playwright dependency. */
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile, writeFile, mkdir, mkdtemp, stat } from 'node:fs/promises';
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
const requestedShots = process.env.DOCS_LAYOUT_SHOTS?.split(',');
await mkdir(output, { recursive: true });
const keys = ['simple', 'safe', 'testable', 'precise', 'extensible', 'fast'];
const source = await readFile(join(root, 'docs/content/docs/index.mdx'), 'utf8');
const sourceMetrics = Object.fromEntries([...source.matchAll(/<section className=\x22example-panel\x22 id=\x22panel-([^\x22]+)\x22[^>]*>([\s\S]*?)<\/section>/g)].map(([, key, body]) => {
  const visible = body.replace(/<div hidden>[\s\S]*?<\/div>/g, '');
  const programs = [...visible.matchAll(/^\x60{3}(?:i|scheme)\b[^\n]*\n([\s\S]*?)^\x60{3}/gm)].flatMap((match) => match[1].trimEnd().split('\n'));
  const commands = [...visible.matchAll(/<pre className=\x22host-command\x22>\{([^\n]+)\}<\/pre>/g)].flatMap((match) => JSON.parse(match[1]).split('\n'));
  const lines = [...programs, ...commands];
  const widest = Math.max(...lines.map((line) => line.replaceAll('\t', '  ').length));
  assert.ok(lines.length <= (key === 'simple' ? 15 : 14) && widest <= 64, `${key}: ${lines.length} lines / ${widest} columns`);
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
    if ((await stat(file)).isDirectory()) file = join(file, 'index.html');
    const body = await readFile(file);
    res.writeHead(200, { 'content-type': mime[extname(file)] ?? 'application/octet-stream' });
    res.end(body);
  } catch { res.writeHead(404); res.end('Not found'); }
});
await new Promise((done) => server.listen(0, '127.0.0.1', done));
let browser;
const measurements = [];
const pageErrors = [];
const screenshotFiles = [];
const guideMeasurements = [];
async function blockInsets(page, selector) {
  return page.locator(selector).evaluate((scope) => [...scope.querySelectorAll('pre')]
    .filter((pre) => pre.getClientRects().length && !pre.closest('[hidden], details:not([open])'))
    .flatMap((pre) => {
      const walker = document.createTreeWalker(pre, NodeFilter.SHOW_TEXT);
      let node;
      while ((node = walker.nextNode())) {
        const offset = node.textContent.search(/\S/);
        if (offset < 0) continue;
        const range = document.createRange();
        range.setStart(node, offset); range.setEnd(node, offset + 1);
        const box = pre.closest('figure') ?? pre;
        return [{ title: box.querySelector('figcaption')?.textContent ?? pre.textContent.trim().split('\n')[0],
          leftOffset: range.getBoundingClientRect().left - box.getBoundingClientRect().left }];
      }
      return [];
    }));
}
async function screenshots(page, prefix, key, directory = output) {
  if (requestedShots && !requestedShots.includes(key)) return;
  await mkdir(directory, { recursive: true });
  await page.evaluate(() => window.scrollTo(0, 0));
  for (const mode of ['first', 'full']) {
    const path = join(directory, `${prefix}-${mode}-${key}.png`);
    await page.screenshot({ path, fullPage: mode === 'full' });
    screenshotFiles.push(path);
  }
}
try {
  browser = await chromium.launch({ executablePath, headless: true, args: ['--no-sandbox', '--disable-dev-shm-usage', '--force-prefers-reduced-motion'] });
  for (const [width, height] of [[1440, 900], [1280, 800], [1920, 1080], [390, 844]]) {
    for (const [theme, javaScriptEnabled] of [['light', true], ['dark', true], ['light', false]]) {
      const context = await browser.newContext({ viewport: { width, height }, colorScheme: theme, javaScriptEnabled, reducedMotion: 'reduce' });
      const page = await context.newPage();
      page.on('pageerror', (error) => pageErrors.push(String(error)));
      await page.goto(`http://127.0.0.1:${server.address().port}/`, { waitUntil: 'networkidle' });
      await page.evaluate(() => document.fonts.ready);
      const prefix = `${width}x${height}-${theme}`;
      const screenshotDirectory = javaScriptEnabled ? output : join(output, 'no-js');
      assert.equal(await page.locator('#nd-docs-layout').count(), 1);
      assert.equal(await page.locator('.home-actions, #nd-home-layout, a[href^="#"]').count(), 0);
      assert.equal(await page.locator('meta[name="astro-view-transitions-enabled"]').count(), 0);
      assert.equal(await page.locator('link[rel="canonical"]').getAttribute('href'), 'https://i.nmnm.cc/');
      if (width > 800) {
        assert.ok(await page.locator('#nd-sidebar').isVisible());
        const homeLink = page.locator('#nd-sidebar a[href="/"]').filter({ hasText: 'Home' });
        assert.ok(await homeLink.isVisible());
        assert.equal(await homeLink.getAttribute('aria-current'), 'page');
      }
      assert.ok(await page.locator('#choice-simple').isChecked());
      assert.equal(await page.locator('.example-panel:visible').count(), 1);
      const sourcePadding = await page.locator('#panel-simple code.language-i .line').first().evaluate((line) => parseFloat(getComputedStyle(line).paddingLeft));
      assert.ok(sourcePadding > 0, `${prefix}: source padding must be positive`);
      let baseline;
      for (const key of keys) {
        await page.locator(`label[for="choice-${key}"]`).click();
        const panel = page.locator(`#panel-${key}`);
        await panel.waitFor({ state: 'visible' });
        assert.equal(await page.locator('.example-panel:visible').count(), 1);
        const data = await panel.evaluate((panel) => {
          const rect = (el) => { const r = el.getBoundingClientRect(); return { left: r.left, right: r.right, top: r.top + scrollY, bottom: r.bottom + scrollY, height: r.height, width: r.width }; };
          const blocks = [...panel.querySelectorAll('figure')].filter((figure) => !figure.closest('[hidden]') && !figure.closest('details:not([open])')).map((figure) => {
            const pre = figure.querySelector('pre'), region = figure.querySelector('[role="region"]');
            const caption = figure.querySelector('figcaption');
            return { title: caption?.textContent, captionFits: !caption || caption.scrollWidth <= caption.clientWidth + 1, output: !!figure.querySelector('code.language-text'), scrollWidth: region.scrollWidth, clientWidth: region.clientWidth, whiteSpace: getComputedStyle(pre).whiteSpace, overflowX: getComputedStyle(region).overflowX, fontSize: getComputedStyle(pre).fontSize, tabSize: getComputedStyle(pre).tabSize, maxHeight: getComputedStyle(region).maxHeight };
          });
          const intro = rect(document.querySelector('.home-intro')), display = rect(document.querySelector('.home-display'));
          const sidebar = document.querySelector('#nd-sidebar');
          const canvas = document.createElement('canvas');
          canvas.width = canvas.height = 1;
          const ctx = canvas.getContext('2d', { willReadFrequently: true });
          const rgba = (color) => { ctx.clearRect(0, 0, 1, 1); ctx.fillStyle = color; ctx.fillRect(0, 0, 1, 1); return [...ctx.getImageData(0, 0, 1, 1).data]; };
          const blend = (front, back) => front.slice(0, 3).map((n, i) => n * front[3] / 255 + back[i] * (1 - front[3] / 255));
          const luminance = (rgb) => rgb.map((n) => n / 255).map((n) => n <= .04045 ? n / 12.92 : ((n + .055) / 1.055) ** 2.4).reduce((sum, n, i) => sum + n * [.2126, .7152, .0722][i], 0);
          const contrast = [...document.querySelectorAll('.home-page h1, .home-eyebrow, .choice-title, .choice-description, .dark #nd-sidebar a, .dark #nd-sidebar button')].filter((el) => el.getClientRects().length && el.textContent.trim()).map((el) => {
            const ancestors = []; for (let p = el; p; p = p.parentElement) ancestors.unshift(p);
            let bg = [255, 255, 255];
            for (const p of ancestors) bg = blend(rgba(getComputedStyle(p).backgroundColor), bg);
            const fg = blend(rgba(getComputedStyle(el).color), bg);
            const a = luminance(fg), b = luminance(bg);
            return { text: el.textContent, ratio: (Math.max(a, b) + .05) / (Math.min(a, b) + .05) };
          });
          const fonts = [...document.fonts].filter((font) => font.status === 'loaded').map((font) => font.family.replaceAll('"', ''));
          const fontRequests = performance.getEntriesByType('resource').filter((r) => /\.(?:woff2?|ttf|otf)(?:\?|$)/.test(r.name)).map((r) => r.name);
          const labels = [...document.querySelectorAll('.home-choice')].map(rect);
          const hostRows = [...panel.querySelectorAll('.host-run')].map((row) => ({ box: rect(row), command: rect(row.querySelector('.host-command')), output: rect(row.querySelector('figure:has(code.language-text)')), commandText: rect(row.querySelector('.host-command pre')), outputText: rect(row.querySelector('figure:has(code.language-text) pre')) }));
          return { scrollHeight: document.documentElement.scrollHeight, scrollWidth: document.documentElement.scrollWidth, clientWidth: document.documentElement.clientWidth, stage: rect(document.querySelector('.example-stage')), sidebar: sidebar ? rect(sidebar) : null, intro, display, labels, hostRows, panel: rect(panel), contentTop: Math.min(intro.top, display.top), contentBottom: Math.max(intro.bottom, display.bottom), contrast, fonts, fontRequests, blocks };
        });
        const label = `${prefix} ${key}`;
        data.insets = await blockInsets(page, '.home-page');
        for (const block of data.insets) assert.ok(block.leftOffset + .01 >= sourcePadding,
          `${label}: ${block.title} text inset ${block.leftOffset}px is less than source padding ${sourcePadding}px`);
        assert.ok(data.scrollHeight <= (width > 800 ? height : height * 1.5), `${label}: scrollHeight ${data.scrollHeight}`);
        assert.equal(data.scrollWidth, data.clientWidth, `${label}: horizontal page overflow`);
        assert.ok(data.fonts.length > 0 && data.fonts.every((font) => /^(?:DM Sans|DM Mono)$/.test(font)), `${label}: unexpected loaded font ${data.fonts}`);
        assert.ok(data.fontRequests.every((url) => url.startsWith(`http://127.0.0.1:${server.address().port}/`) && /dm-(sans|mono)/.test(url)), `${label}: external or non-DM web font`);
        for (const text of data.contrast) assert.ok(text.ratio >= 4.5, `${label}: low text contrast ${text.ratio.toFixed(2)}: ${text.text}`);
        assert.ok(data.panel.top >= data.stage.top - 1 && data.panel.bottom <= data.stage.bottom + 1, `${label}: example exceeds its fixed stage`);
        if (width > 800) {
          assert.ok(data.contentBottom <= height, `${label}: content is below viewport`);
          assert.ok(data.contentTop <= 32, `${label}: content should start near the top`);
          assert.ok(Math.abs(data.intro.top - data.display.top) <= 1 && Math.abs(data.panel.top - data.display.top) <= 1, `${label}: columns and example should be top-aligned`);
          assert.equal(data.hostRows.length, key === 'fast' ? 3 : 0, `${label}: paired command/output rows`);
          for (const row of data.hostRows) {
            assert.ok(Math.abs(row.command.top - row.output.top) <= 1, `${label}: command/output tops differ`);
            assert.ok(Math.abs(row.command.height - row.output.height) <= 1, `${label}: command/output heights differ`);
            assert.ok(Math.abs(row.commandText.top - row.outputText.top) <= 1, `${label}: command text is not on the output text line`);
          }
          const gaps = data.hostRows.slice(1).flatMap((row, i) => ['command', 'output'].map((side) => row[side].top - data.hostRows[i][side].bottom));
          assert.ok(gaps.every((gap) => gap >= 0 && Math.abs(gap - gaps[0]) <= 1), `${label}: command/output row gaps differ: ${gaps}`);
          assert.ok(data.intro.left >= data.sidebar.right && data.intro.right <= data.display.left, `${label}: sidebar and columns overlap`);
          assert.ok(data.display.right <= data.clientWidth, `${label}: content exceeds viewport`);
          for (let i = 1; i < data.labels.length; i++) assert.ok(data.labels[i].top >= data.labels[i - 1].bottom, `${label}: selector text overlaps`);
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
        } else baseline = data;
        await screenshots(page, prefix, key, screenshotDirectory);
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
      for (const block of await blockInsets(page, '.home-page')) assert.ok(block.leftOffset + .01 >= sourcePadding, `${prefix}: expanded macro block lost left padding`);
      assert.ok(expanded.bottom <= expanded.stage + 1, `${prefix}: open macro definition exceeds stage`);
      assert.ok(expanded.height <= (width > 800 ? height : height * 1.5));
      if (width > 800) {
        const definition = await page.locator('.macro-definition [role="region"]').evaluate((el) => ({ width: el.clientWidth, scroll: el.scrollWidth }));
        assert.ok(definition.scroll <= definition.width + 1, `${prefix}: macro definition source is clipped`);
      }
      await screenshots(page, prefix, 'definition', screenshotDirectory);
      if (width < 801 && javaScriptEnabled) {
        await page.locator('label[for="choice-simple"]').click();
        await page.getByRole('button', { name: 'Open Sidebar', exact: true }).click();
        const drawer = page.locator('#nd-sidebar-mobile');
        await drawer.waitFor({ state: 'visible' });
        await drawer.evaluate((el) => Promise.allSettled(el.getAnimations({ subtree: true }).map((animation) => animation.finished)));
        assert.ok(await drawer.getByRole('link', { name: 'Home', exact: true }).isVisible());
        const bounds = await drawer.boundingBox();
        assert.ok(bounds.x >= 0 && bounds.x + bounds.width <= width, `${prefix}: mobile sidebar exceeds viewport: ${JSON.stringify(bounds)}`);
        assert.ok(await page.evaluate(() => document.documentElement.scrollWidth === document.documentElement.clientWidth), `${prefix}: opening the sidebar creates horizontal scroll`);
        await drawer.getByRole('button', { name: 'Getting started', exact: true }).click();
        await drawer.getByRole('link', { name: 'Install', exact: true }).waitFor({ state: 'visible' });
        await drawer.evaluate((el) => Promise.allSettled(el.getAnimations({ subtree: true }).map((animation) => animation.finished)));
        await screenshots(page, prefix, 'sidebar');
        await page.keyboard.press('Escape');
        await drawer.waitFor({ state: 'hidden' });
        await page.getByRole('button', { name: 'Open Sidebar', exact: true }).click();
        await drawer.waitFor({ state: 'visible' });
        await drawer.getByRole('button', { name: 'Close Sidebar', exact: true }).click();
        await drawer.waitFor({ state: 'hidden' });
        await page.getByRole('button', { name: 'Open Sidebar', exact: true }).click();
        await drawer.getByRole('link', { name: 'Install', exact: true }).click();
        await page.waitForURL(`http://127.0.0.1:${server.address().port}/docs/getting-started/install`);
        await drawer.waitFor({ state: 'hidden' });
      }
      await context.close();
    }
  }
  // Ordinary command fences must retain the same padding outside the home page.
  for (const [width, height] of [[1440, 900], [390, 844]]) {
    for (const theme of ['light', 'dark']) {
      const context = await browser.newContext({ viewport: { width, height }, colorScheme: theme, reducedMotion: 'reduce' });
      const page = await context.newPage();
      page.on('pageerror', (error) => pageErrors.push(String(error)));
      await page.goto(`http://127.0.0.1:${server.address().port}/docs/getting-started/first-program`, { waitUntil: 'networkidle' });
      await page.evaluate(() => document.fonts.ready);
      const padding = await page.locator('#nd-page code.language-i .line').first().evaluate((line) => parseFloat(getComputedStyle(line).paddingLeft));
      const insets = await blockInsets(page, '#nd-page');
      assert.ok(insets.length > 0 && insets.every((block) => block.leftOffset + .01 >= padding), `${width} ${theme}: guide code or command padding missing`);
      assert.ok(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth), `${width} ${theme}: guide page overflow`);
      guideMeasurements.push({ viewport: [width, height], theme, padding, insets });
      await screenshots(page, `${width}x${height}-${theme}`, 'first-program');
      await context.close();
    }
  }
  // The canonical entry is searchable, and ordinary docs link back to it.
  const nav = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  nav.on('pageerror', (error) => pageErrors.push(String(error)));
  const address = `http://127.0.0.1:${server.address().port}`;
  await nav.goto(`${address}/docs/`);
  await nav.waitForURL(`${address}/`);
  await nav.locator('#choice-simple').waitFor();
  await nav.goto(`${address}/docs/getting-started`, { waitUntil: 'networkidle' });
  await nav.locator('astro-island[component-export="Docs"]:not([ssr])').waitFor({ state: 'attached' });
  const previous = nav.locator('[data-page-chrome] a[href="/"]');
  assert.equal(await previous.count(), 1);
  await previous.click();
  await nav.waitForURL(`${address}/`);
  await nav.locator('#choice-simple').waitFor();
  assert.ok(await nav.locator('#nd-sidebar a[href="/"]').filter({ hasText: 'Home' }).isVisible());
  await nav.locator('.home-doc-page').waitFor({ state: 'visible' });
  assert.ok(await nav.evaluate(() => document.documentElement.scrollHeight <= 900));
  await nav.goto(`${address}/docs/getting-started/install`, { waitUntil: 'networkidle' });
  await nav.locator('astro-island[component-export="Docs"]:not([ssr])').waitFor({ state: 'attached' });
  await nav.keyboard.press('Control+k');
  await nav.locator('[role="dialog"] input').fill('The language for the rest of us');
  await nav.locator('[role="option"][id="fd-search-option-/"]').click();
  await nav.waitForURL(`${address}/`);
  await nav.locator('.home-doc-page').waitFor({ state: 'visible' });
  assert.equal(await nav.locator('a[href^="#"]').count(), 0);
  await nav.close();
  assert.deepEqual(pageErrors, []);
  await writeFile(join(output, 'measurements.json'), JSON.stringify({ proseWords, sourceMetrics, measurements, guideMeasurements, screenshotFiles }, null, 2));
  console.log(`${measurements.length} layouts passed; ${proseWords} prose words; screenshots: ${output}`);
} finally {
  await browser?.close();
  await new Promise((done) => server.close(done));
}
