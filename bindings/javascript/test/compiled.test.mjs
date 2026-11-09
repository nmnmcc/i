// The JavaScript host with a module built by the compiler:
// tests/browser/fixtures/compiled.wasm, from tests/browser/fixtures/compiled
// (regenerate it with `bash scripts/build-abi-fixture.sh`).
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { compiledImports } from "../../../tests/browser/fixture-abi.mjs";
import { instantiate, iLangError, iLangStop } from "../ilang.mjs";

const compiled = await readFile(new URL("../../../tests/browser/fixtures/compiled.wasm", import.meta.url));
const guest = (printed = [], overrides = {}) => instantiate(compiled, { imports: compiledImports(iLangError, printed, overrides) });

test("a compiled module lists the host functions that its entry points need", async () => {
  const value = await guest();
  assert.deepEqual(value.abi.entry, { name: "app/app.main", async: true });
  assert.equal(value.abi.exports["app/app.greet"].async, false);
  assert.equal(value.abi.exports["app/app.wait_sum"].async, true);
  assert.deepEqual(value.abi.services["app:storage/v1"].read, { params: ["string"], result: "string", errors: ["string"], async: false });
  assert.deepEqual(value.abi.services["app:clock/v1"].tick, { params: ["int"], result: "int", errors: [], async: true });
  assert.deepEqual(value.abi.externals["app:math/v1"].hypot, { params: ["float", "float"], result: "float", errors: [], async: false });
});

test("sync host services and external functions of a compiled module", async () => {
  const printed = [];
  const value = await guest(printed);
  assert.equal(value.call("app/app.greet", ["é"]), 2n);
  assert.deepEqual(printed, ["hello é"]);
  assert.equal(value.call("app/app.lookup", ["name"]), "i");
  assert.throws(() => value.call("app/app.lookup", ["other"]), (error) => error instanceof iLangError && error.value === "no other");
  assert.equal(value.call("app/app.distance", [3, 4]), 5);
  assert.equal(value.unusable, null);
});

test("async host services of a compiled module wait for the host", async () => {
  const printed = [];
  const value = await guest(printed);
  assert.equal(await value.callAsync("app/app.wait_sum", [1.5, 2]), 3.5);
  await assert.rejects(value.callAsync("app/app.wait_sum", [1, -1]), (error) => error instanceof iLangError && error.value === "negative");
  await value.run();
  assert.deepEqual(printed, ["main 42 1.5"]);
  assert.equal(value.unusable, null);
});

test("the module traps when the host answers a function it does not wait for", async () => {
  const value = await guest();
  const waiting = value.callAsync("app/app.wait_sum", [1, 2]);
  // The module waits for `later`, not for `tick` (B.5).
  assert.throws(() => value.exports["ilang_resume:app:clock/v1:tick"](0, 0, 0), WebAssembly.RuntimeError);
  assert.equal(await waiting, 3);
  const idle = await guest();
  assert.throws(() => idle.exports["ilang_resume:app:clock/v1:later"](0, 1, 0, 0), WebAssembly.RuntimeError);
});

test("an answer of the host that is not correct stops the program", async () => {
  const value = await guest([], { later: async () => NaN });
  await assert.rejects(value.callAsync("app/app.wait_sum", [1, 2]), (error) => error instanceof iLangStop && error.stopMessage === "host: value is not correct");
  assert.ok(value.unusable);
});
