import { test, expect } from '@playwright/test';
import { execFileSync } from 'node:child_process';
import { resolve } from 'node:path';
const root = resolve(import.meta.dirname, '../..');
test.beforeAll(() => {
  execFileSync(resolve(root, 'target/debug/i'), ['build', '--project', resolve(root, 'tests/browser/fixtures/http-fetch'), '--target', 'js', '--opt', '0'], { stdio: 'pipe' });
});
test('http.get uses the browser fetch provider', async ({ page }) => {
  await page.goto('/tests/browser/harness.html');
  const text = await page.evaluate(async () => {
    const { instantiate } = await import('/bindings/javascript/ilang.mjs');
    const guest = await instantiate(new URL('/tests/browser/fixtures/http-fetch/build/http_fetch.wasm', location.href), { standardServices: true });
    return guest.callAsync('http_fetch/main.get', [new URL('/tests/browser/fixtures/http-fetch/reply.txt', location.href).href]);
  });
  expect(text).toBe('browser http ok\n');
});
