// The JavaScript host with a module built by the compiler:
// tests/browser/fixtures/compiled.wasm, from tests/browser/fixtures/compiled
// (regenerate it with `bash scripts/build-abi-fixture.sh`).
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { compiledImports, checkOpaque } from "../../../tests/browser/fixture-abi.mjs";
import { instantiate, GuestError, Stop } from "../ilang.mjs";

const compiled = await readFile(new URL("../../../tests/browser/fixtures/compiled.wasm", import.meta.url));
const guest = (printed = [], overrides = {}) => instantiate(compiled, { imports: compiledImports(GuestError, printed, overrides) });

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
  assert.throws(() => value.call("app/app.lookup", ["other"]), (error) => error instanceof GuestError && error.value === "no other");
  assert.equal(value.call("app/app.distance", [3, 4]), 5);
  assert.equal(value.unusable, null);
});

test("async host services of a compiled module wait for the host", async () => {
  const printed = [];
  const value = await guest(printed);
  assert.equal(await value.callAsync("app/app.wait_sum", [1.5, 2]), 3.5);
  await assert.rejects(value.callAsync("app/app.wait_sum", [1, -1]), (error) => error instanceof GuestError && error.value === "negative");
  await value.run();
  assert.deepEqual(printed, ["main 42 1.5"]);
  assert.equal(value.unusable, null);
});

test("the module traps when the host answers a request it does not wait for", async () => {
  for (const [resume, args] of [
    // The module waits for request 0 of `later`, not of `tick`.
    ["tick", [0, 0, 0, 0]],
    // Nor for another id of `later`.
    ["later", [5, 0, 1, 0, 0]],
  ]) {
    const value = await guest([], { later: () => new Promise(() => {}) });
    void value.callAsync("app/app.wait_sum", [1, 2]);
    assert.throws(() => value.exports[`ilang_resume:app:clock/v1:${resume}`](...args), WebAssembly.RuntimeError);
    // Keep the work pending: no SDK resume may use this instance after a trap.
  }
  const value = await guest();
  await value.callAsync("app/app.wait_sum", [1, 2]);
  // Requests 0 and 1 were answered: their ids are stale now.
  assert.throws(() => value.exports["ilang_resume:app:clock/v1:later"](1, 0, 1, 0, 0), WebAssembly.RuntimeError);
  const idle = await guest();
  assert.throws(() => idle.exports["ilang_resume:app:clock/v1:later"](0, 0, 1, 0, 0), WebAssembly.RuntimeError);
});

test("an answer of the host that is not correct stops the program in the module", async () => {
  // By default the module checks the answer, and the stop has the position
  // of the call.
  const value = await guest([], { later: async () => NaN });
  await assert.rejects(value.callAsync("app/app.wait_sum", [1, 2]), (error) => error instanceof Stop && error.stopMessage === "host: value is not correct\napp/app:52:2");
  assert.ok(value.unusable);
  const sync = await instantiate(compiled, { imports: { ...compiledImports(GuestError), "app:math/v1": { hypot: () => Infinity } } });
  assert.throws(() => sync.call("app/app.distance", [3, 4]), (error) => error instanceof Stop && error.stopMessage === "host: value is not correct\napp/app:48:35");
  // With `checkReplies`, the host stops the program, with no positions.
  const checked = await instantiate(compiled, { imports: compiledImports(GuestError, [], { later: async () => NaN }), checkReplies: true });
  await assert.rejects(checked.callAsync("app/app.wait_sum", [1, 2]), (error) => error instanceof Stop && error.stopMessage === "host: value is not correct");
  const checkedSync = await instantiate(compiled, { imports: { ...compiledImports(GuestError), "app:math/v1": { hypot: () => Infinity } }, checkReplies: true });
  assert.throws(() => checkedSync.call("app/app.distance", [3, 4]), (error) => error instanceof Stop && error.stopMessage === "host: value is not correct");
});

test("an exception of a host function goes to the caller", async () => {
  const failure = new Error("the storage is gone");
  const value = await guest([], { read: () => { throw failure; } });
  assert.throws(() => value.call("app/app.lookup", ["name"]), (error) => error === failure);
  assert.match(value.unusable, /the storage is gone/);
});

// Distinct nominal constructors are different values, also when the host
// passes one object through both declarations.
test("compiled opaque imports and exports preserve nominal identity", async () => {
  const reference = {};
  for (const opaque of [reference, null]) {
    const value = await guest([], { opaque });
    assert.equal(await checkOpaque(value, reference, {}), true);
  }
});

// Consumes the real compiler's separate plan, keeps restricted callbacks in
// one invocation, and disposes every created resource.
test("compiler binding plan authorizes context registrations on the JS host", async () => {
  const { authorizeBindings } = await import("../ilang.mjs");
  const plan = JSON.parse(await readFile(new URL("../../../tests/browser/fixtures/compiled.bindings.json", import.meta.url), "utf8"));
  // These checked-in fixture bytes and compiler output are the test embedding's
  // explicit trust path. Production callers authenticate their build outputs.
  const bindings = await authorizeBindings(compiled, plan, () => true);
  const contexts = new Set();
  let created = 0;
  let disposed = 0;
  const contextImports = Object.fromEntries(Object.entries(compiledImports(GuestError)).map(([namespace, fields]) => [namespace, Object.fromEntries(Object.entries(fields).map(([name, fn]) => [name, (context, ...args) => {
    assert.equal(context.live, true);
    contexts.add(context);
    created++;
    context.insert("fixture", {}, () => { disposed++; });
    return fn(...args);
  }]))]));
  const value = await instantiate(bindings.module, { contextImports, bindings });
  assert.equal(value.call("app/app.distance", [3, 4]), 5);
  assert.equal(await value.callAsync("app/app.wait_sum", [1.5, 2]), 3.5);
  assert.equal(contexts.size, 2);
  assert.equal(disposed, created);
  assert.ok([...contexts].every((context) => !context.live));
});

test("ABI 5 Int boundaries cross imports, exports, async replies and errors", async () => {
  const { checkIntegers } = await import("../../../tests/browser/fixture-abi.mjs");
  const value = await guest([], { tick: async n => n });
  assert.equal(value.abi.version, 5);
  assert.equal(await checkIntegers(value, GuestError), 13);
  assert.equal(value.unusable, null);
});

test("the module rejects an inline Int whose sign does not match its value", async () => {
  for (const [sign, n] of [[1, -1], [-1, 1], [1, 0], [2, 2]]) {
    const value = await guest();
    assert.throws(() => value.exports["app/ints.echo"](sign, n, 0), WebAssembly.RuntimeError);
  }
});
