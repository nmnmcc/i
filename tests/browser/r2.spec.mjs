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
  expect(results[0].results).toHaveLength(9);
  expect(results[0].expected).toEqual(results[1].expected);
  expect(results.map((r) => r.repetitions)).toEqual([200, 200]);
});
