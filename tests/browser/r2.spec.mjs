import { expect, test } from "@playwright/test";

test("R2 completion gates, ownership, quiescence and 200 seeded replays at O0/O2", async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto("/tests/browser/harness.html");
  const results = await page.evaluate(async () => {
    const sdk = await import("/bindings/javascript/ilang.mjs");
    const { checkR2 } = await import("/tests/browser/r2-support.mjs");
    const results = [];
    for (const level of [0, 2]) {
      const bytes = await (await fetch(`/tests/browser/fixtures/r2-o${level}.wasm`)).arrayBuffer();
      const plan = await (await fetch(`/tests/browser/fixtures/r2-o${level}.bindings.json`)).json();
      results.push(await checkR2(sdk, bytes, plan));
    }
    return results;
  });
  expect(results[0].results).toHaveLength(10);
  expect(results[0].expected).toEqual(results[1].expected);
  expect(results.map((r) => r.repetitions)).toEqual([200, 200]);
});

test("every runnable R2 spec body passes at O0/O2", async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto("/tests/browser/harness.html");
  const result = await page.evaluate(async () => {
    const sdk = await import("/bindings/javascript/ilang.mjs");
    const { default: cases } = await import("/tests/browser/fixtures/r2-laws/cases.mjs");
    const { checkR2Laws } = await import("/tests/browser/r2-laws.mjs");
    const passed = [];
    for (const level of [0, 2]) {
      const bytes = await (await fetch(`/tests/browser/fixtures/r2-laws/laws-o${level}.wasm`)).arrayBuffer();
      const plan = await (await fetch(`/tests/browser/fixtures/r2-laws/laws-o${level}.bindings.json`)).json();
      passed.push(await checkR2Laws(sdk, bytes, plan, cases));
    }
    return { count: cases.length, passed };
  });
  expect(result.passed).toEqual([result.count, result.count]);
});
