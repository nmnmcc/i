import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { ResourceHost, instantiate, authorizeBindings, GuestError } from "../ilang.mjs";
import { customAbi, customImports, hostImports, compiledImports } from "../../../tests/browser/fixture-abi.mjs";

const fixture = await readFile(new URL("../../../tests/browser/fixtures/host-abi.wasm", import.meta.url));
const custom = await readFile(new URL("../../../tests/browser/fixtures/custom-imports.wasm", import.meta.url));

test("BG review 1: approval retains the complete plan despite source mutation", async () => {
  const bytes = await readFile(new URL("../../../tests/browser/fixtures/compiled.wasm", import.meta.url));
  const plan = JSON.parse(await readFile(new URL("../../../tests/browser/fixtures/compiled.bindings.json", import.meta.url), "utf8"));
  const bindings = await authorizeBindings(bytes, plan, (d) => d.declaration !== "get_b");
  plan.declarations = plan.declarations.filter((d) => d.declaration !== "get_b");
  const imports = compiledImports(GuestError);
  delete imports["app:opaque/v1"].get;
  await assert.rejects(instantiate(bindings.module, { imports, bindings, contextImports: { "app:opaque/v1": { get: () => null } } }), /ungranted host import.*get_b/);
});

test("BG review 2: dispose during status 2 prevents continuation", async () => {
  const guest = await instantiate(fixture, { imports: hostImports(GuestError) });
  const pending = guest.callAsync("app/async.yielding", [3]);
  guest.dispose();
  await assert.rejects(pending, /disposed|ended/);
});

test("BG review 3: synchronous opaque objects are never inspected for then or finally", async () => {
  const guest = await instantiate(fixture, { imports: hostImports(GuestError) });
  const references = [
    { then() { assert.fail("must not execute opaque then"); } },
    Object.defineProperties({}, { then: { get() { assert.fail("must not read opaque then"); } }, finally: { get() { assert.fail("must not read opaque finally"); } } }),
    new Proxy({}, { get() { assert.fail("must not inspect opaque result"); } }),
  ];
  for (const reference of references) assert.equal(guest.call("opaque_roundtrip", [reference]), reference);
  const ref = references[1];
  const imported = await instantiate(custom, { abi: customAbi, imports: customImports(GuestError, { echo_ref: () => ref }) });
  assert.equal(imported.call("echo_ref", [{}]), ref);
});

test("BG review 4: cancellation cannot acquire, commit or deliver after teardown", async () => {
  const context = new ResourceHost().context();
  const id = context.insert("file", {}, () => {});
  const lease = context.acquire(id, "file", "read");
  const provisional = context.provisional(id, "file");
  const closed = (error) => error.code === "closed";
  let checked = false;
  context.ownRequest(() => {
    assert.equal(context.live, false);
    assert.throws(() => context.acquire(id, "file", "write"), closed);
    assert.throws(() => lease.commit(42), closed);
    assert.throws(() => provisional.deliver(), closed);
    checked = true;
  });
  await context.teardown();
  assert.equal(checked, true);
});

test("BG review 5: cancellation failure cannot skip other cancellations or cleanup", async () => {
  const context = new ResourceHost().context();
  const calls = [];
  const id = context.insert("file", {}, () => { calls.push("cleanup"); });
  context.ownRequest(() => { calls.push("first cancel"); throw new Error("cancel failed"); });
  context.ownRequest(() => { calls.push("second cancel"); });
  await context.teardown();
  assert.deepEqual(calls, ["first cancel", "second cancel", "cleanup"]);
  assert.throws(() => context.acquire(id, "file", "read"), /closed/);
});

test("BG review 5: cleanup and abort listener failures are collected without skipping steps", async () => {
  const host = new ResourceHost();
  const context = host.context();
  const sync = new Error("sync cleanup failed");
  const async = new Error("async cleanup failed");
  const wake = new Error("wake failed");
  const cancel = new Error("cancel failed");
  let cleaned = 0;
  const a = context.insert("file", {}, () => { cleaned++; throw sync; });
  context.insert("file", {}, () => { cleaned++; return Promise.reject(async); });
  const lease = context.acquire(a, "file", "read");
  lease.signal.addEventListener("abort", () => { throw wake; });
  let onabortRan = false;
  lease.signal.onabort = () => { onabortRan = true; };
  context.ownRequest(() => { throw cancel; });
  const ended = context.teardown();
  assert.equal(context.teardown(), ended);
  await ended;
  assert.equal(cleaned, 2);
  assert.equal(onabortRan, true);
  const errors = host.takeCleanupErrors();
  for (const error of [sync, async, wake, cancel]) assert.equal(errors.filter((failure) => failure.error === error).length, 1);
  assert.deepEqual(host.takeCleanupErrors(), []);
});

test("BG review 4/5: reentrant teardown joins every resource before running cleanup", async () => {
  const host = new ResourceHost();
  const context = host.context();
  let finish;
  const id = context.insert("file", {}, () => new Promise((resolve) => { finish = resolve; }));
  let reentered;
  let close;
  context.ownRequest(() => { reentered = context.teardown(); close = context.close(id, "file"); });
  const ended = context.teardown();
  assert.equal(ended, reentered);
  let done = false;
  ended.then(() => { done = true; });
  await Promise.resolve();
  assert.equal(done, false);
  finish();
  await Promise.all([ended, close]);
  assert.equal(done, true);
  assert.deepEqual(host.takeCleanupErrors(), []);
});

const lifecycle = await readFile(new URL("../../../tests/browser/fixtures/bg-lifecycle.wasm", import.meta.url));
const sdk = await import("../ilang.mjs");
const { bgLifecycleGuest, bgCounter: counter } = await import("../../../tests/browser/bg-review-support.mjs");
const lifecycleGuest = (capture, wait) => bgLifecycleGuest(sdk, lifecycle, capture, wait);

test("BG review 2: no-request continuations and immediate results stay behind the context gate", async () => {
  for (const operation of ["callAsync", "run", "runEntryPoint"]) {
    let context;
    const guest = await lifecycleGuest((current) => { context = current; });
    const pending = operation === "callAsync" ? guest.callAsync("runnable") : operation === "run" ? guest.run() : guest.runEntryPoint("entry");
    context.teardown();
    await assert.rejects(pending, /ended/);
    assert.equal(counter(guest, 0), 0);
    assert.equal(counter(guest, 4), 0);
    assert.ok(guest.unusable);
    assert.throws(() => guest.call("sync"), /cannot be used/);
  }
  let context;
  const guest = await lifecycleGuest((current) => { context = current; });
  const pending = guest.callAsync("immediate");
  context.teardown();
  await assert.rejects(pending, /ended/);
  assert.equal(counter(guest, 4), 0);
});

test("BG review 2: cancellation at status zero blocks the later result export", async () => {
  const { requestSignal } = await import("../ilang.mjs");
  const guest = await lifecycleGuest(() => {}, (context) => {
    requestSignal().addEventListener("abort", () => context.teardown());
    return new Promise(() => {});
  });
  await assert.rejects(guest.callAsync("leftover"), /ended/);
  assert.equal(counter(guest, 4), 0);
});

test("BG review audit: sync callbacks, allocators and each free are gated", async () => {
  for (const mode of [0, 16, 20]) {
    const guest = await lifecycleGuest((context) => { if (mode === 0) context.teardown(); });
    if (mode) new DataView(guest.exports.memory.buffer).setUint32(mode, 1, true);
    assert.throws(() => mode ? guest.call("buffers", ["first", "second"]) : guest.call("sync"), /ended/);
    if (mode === 20) assert.equal(counter(guest, 24), 1);
    else assert.equal(counter(guest, 12), 0);
  }
});

test("BG review audit: cleanup cannot reenter a new invocation while its old one ends", async () => {
  let guest;
  let refusal;
  let registered = false;
  guest = await lifecycleGuest((context) => {
    if (registered) return;
    registered = true;
    context.ownRequest(() => {
      try { guest.call("sync"); } catch (error) { refusal = error; }
    });
  });
  guest.call("sync");
  assert.match(refusal?.message, /tearing down/);
  assert.equal(counter(guest, 12), 1);
  guest.call("sync");
  assert.equal(counter(guest, 12), 2);
  assert.deepEqual(guest.takeCleanupErrors(), []);
});

test("BG review audit: request abort errors are diagnostic and preserve a successful result", async () => {
  const { requestSignal } = await import("../ilang.mjs");
  const failure = new Error("request listener failed");
  const guest = await lifecycleGuest(() => {}, () => {
    requestSignal().addEventListener("abort", () => { throw failure; });
    return new Promise(() => {});
  });
  await guest.callAsync("leftover");
  assert.equal(counter(guest, 4), 1);
  assert.equal(guest.unusable, null);
  assert.deepEqual(guest.takeCleanupErrors(), [{ phase: "cancel", error: failure }]);
});

test("BG review audit: isolated signals retain AbortSignal brands and listener removal", () => {
  const host = new ResourceHost();
  const controller = host.context().abortController();
  assert.ok(controller.signal instanceof AbortSignal);
  const request = new Request("https://example.invalid", { signal: controller.signal });
  const unused = () => { assert.fail("removed listener ran"); };
  controller.signal.addEventListener("abort", unused);
  controller.signal.removeEventListener("abort", unused);
  let once = 0;
  controller.signal.addEventListener("abort", () => { once++; }, { once: true });
  controller.abort();
  controller.abort();
  assert.equal(request.signal.aborted, true);
  assert.equal(once, 1);
  assert.deepEqual(host.takeCleanupErrors(), []);
});

test("BG review audit: a rejected cancellation callback cannot become an unhandled rejection", async () => {
  const host = new ResourceHost();
  const context = host.context();
  let reject;
  const pending = new Promise((_, fail) => { reject = fail; });
  context.ownRequest(() => pending);
  await context.teardown();
  const failure = new Error("late cancellation failure");
  reject(failure);
  await pending.catch(() => {});
  assert.deepEqual(host.takeCleanupErrors(), [{ phase: "cancel", error: failure }]);
});

test("BG review audit: argument getters cannot bypass the final gate into an export", async () => {
  let context;
  class CapturedHost extends ResourceHost {
    context() { context = super.context(); return context; }
  }
  const guest = await bgLifecycleGuest(sdk, lifecycle, undefined, undefined, new CapturedHost());
  let reads = 0;
  const bits = { get bytes() {
    if (++reads === 3) context.teardown(); // after validation, during lowering
    return new Uint8Array();
  }, bitLength: 0 };
  assert.throws(() => guest.call("bits", [bits]), /ended/);
  assert.equal(counter(guest, 12), 0);
});

test("BG review audit: repeated cancellation callbacks remain independent registrations", async () => {
  const host = new ResourceHost();
  const context = host.context();
  let calls = 0;
  const cancel = () => { calls++; };
  const detach = context.ownRequest(cancel);
  context.ownRequest(cancel);
  detach();
  context.ownRequest(cancel);
  await context.teardown();
  assert.equal(calls, 2);
  assert.deepEqual(host.takeCleanupErrors(), []);
});
