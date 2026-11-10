import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import * as sdk from "../ilang.mjs";
import { checkR2 } from "../../../tests/browser/r2-support.mjs";

const load = async (level) => ({
  bytes: await readFile(new URL(`../../../tests/browser/fixtures/r2-o${level}.wasm`, import.meta.url)),
  plan: JSON.parse(await readFile(new URL(`../../../tests/browser/fixtures/r2-o${level}.bindings.json`, import.meta.url), "utf8")),
});
test("R2 completion gates, ownership, quiescence and 200 seeded replays at O0/O2", { timeout: 120_000 }, async () => {
  let previous;
  for (const level of [0, 2]) {
    const { bytes, plan } = await load(level);
    const result = await checkR2(sdk, bytes, plan);
    assert.equal(result.results.length, 10);
    if (previous) assert.equal(result.expected, previous.expected);
    previous = result;
  }
});
test("pure task switches finish without a host macrotask", async () => {
  for (const level of [0, 2]) {
    const { bytes, plan } = await load(level);
    const bindings = await sdk.authorizeBindings(bytes, plan, () => true);
    const value = await sdk.instantiate(bindings.module, { bindings, imports: {
      "r2:gates/v1": { wait: () => new Promise(() => {}) },
      "r2:control/v1": { release: () => {}, check: () => {} },
    } });
    const original = globalThis.setImmediate;
    let calls = 0;
    globalThis.setImmediate = (...args) => { ++calls; return original(...args); };
    try { assert.equal(await value.callAsync("app/r2.pure_tasks"), 7n); }
    finally { globalThis.setImmediate = original; }
    assert.equal(calls, 0);
  }
});

test("every runnable R2 spec body passes on Node at O0/O2", { timeout: 120_000 }, async () => {
  const { default: cases } = await import("../../../tests/browser/fixtures/r2-laws/cases.mjs");
  const { checkR2Laws } = await import("../../../tests/browser/r2-laws.mjs");
  for (const level of [0, 2]) {
    const bytes = await readFile(new URL(`../../../tests/browser/fixtures/r2-laws/laws-o${level}.wasm`, import.meta.url));
    const plan = JSON.parse(await readFile(new URL(`../../../tests/browser/fixtures/r2-laws/laws-o${level}.bindings.json`, import.meta.url), "utf8"));
    assert.equal(await checkR2Laws(sdk, bytes, plan, cases), cases.length);
  }
});
