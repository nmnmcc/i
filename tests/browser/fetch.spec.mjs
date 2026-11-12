import { test, expect } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
const root = resolve(import.meta.dirname, '../..');
let artifact;
test.beforeAll(({}, info) => {
  artifact = `http_fetch-${info.project.name}.wasm`;
  execFileSync(resolve(root, 'target/debug/i'), ['build', '--project', resolve(root, 'tests/browser/fixtures/http-fetch'), '--target', 'js', '--opt', '0', '--out', resolve(root, 'tests/browser/fixtures/http-fetch/build', artifact)], { stdio: 'pipe' });
});
test('http.get uses the browser fetch provider', async ({ page }) => {
  await page.goto('/tests/browser/harness.html');
  const text = await page.evaluate(async artifact => {
    const { instantiate } = await import('/bindings/javascript/ilang.mjs');
    const guest = await instantiate(new URL('/tests/browser/fixtures/http-fetch/build/' + artifact, location.href), { standardServices: true });
    return guest.callAsync('http_fetch/main.get', [new URL('/tests/browser/fixtures/http-fetch/reply.txt', location.href).href]);
  }, artifact);
  expect(text).toBe('browser http ok\n');
});
