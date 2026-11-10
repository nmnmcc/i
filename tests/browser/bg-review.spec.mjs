import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => { await page.goto("/tests/browser/harness.html"); });

test("BG review: complete plan authorization and opaque values", async ({ page }) => {
  const result = await page.evaluate(async () => {
    const sdk = await import("/bindings/javascript/ilang.mjs");
    const { hostImports, customImports, customAbi, compiledImports } = await import("/tests/browser/fixture-abi.mjs");
    const guest = await sdk.instantiate("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(sdk.GuestError) });
    const reference = Object.defineProperties({}, { then: { get() { throw new Error("read opaque then"); } }, finally: { get() { throw new Error("read opaque finally"); } } });
    const roundtrip = guest.call("opaque_roundtrip", [reference]) === reference;
    const custom = await sdk.instantiate("/tests/browser/fixtures/custom-imports.wasm", { abi: customAbi, imports: customImports(sdk.GuestError, { echo_ref: () => reference }) });
    const imported = custom.call("echo_ref", [{}]) === reference;
    const bytes = await (await fetch("/tests/browser/fixtures/compiled.wasm")).arrayBuffer();
    const plan = await (await fetch("/tests/browser/fixtures/compiled.bindings.json")).json();
    const bindings = await sdk.authorizeBindings(bytes, plan, (d) => d.declaration !== "get_b");
    plan.declarations = plan.declarations.filter((d) => d.declaration !== "get_b");
    const imports = compiledImports(sdk.GuestError);
    delete imports["app:opaque/v1"].get;
    let refusal;
    try { await sdk.instantiate(bindings.module, { bindings, imports, contextImports: { "app:opaque/v1": { get: () => null } } }); } catch (error) { refusal = error.message; }
    return { roundtrip, imported, refused: /ungranted host import.*get_b/.test(refusal) };
  });
  expect(result).toEqual({ roundtrip: true, imported: true, refused: true });
});

test("BG review: every continuation, result, allocation and free is gated", async ({ page }) => {
  const result = await page.evaluate(async () => {
    const sdk = await import("/bindings/javascript/ilang.mjs");
    const { bgLifecycleGuest, bgCounter } = await import("/tests/browser/bg-review-support.mjs");
    const load = (capture, wait) => bgLifecycleGuest(sdk, "/tests/browser/fixtures/bg-lifecycle.wasm", capture, wait);
    const results = [];
    for (const operation of ["callAsync", "run", "runEntryPoint", "immediate"]) {
      let context;
      const guest = await load((current) => { context = current; });
      const pending = operation === "callAsync" ? guest.callAsync("runnable") : operation === "run" ? guest.run() : operation === "runEntryPoint" ? guest.runEntryPoint("entry") : guest.callAsync("immediate");
      context.teardown();
      let refused = false;
      try { await pending; } catch (error) { refused = /ended/.test(error.message); }
      results.push(refused && bgCounter(guest, 0) === 0 && bgCounter(guest, 4) === 0 && !!guest.unusable);
    }
    const leftover = await load(() => {}, (context) => {
      sdk.requestSignal().addEventListener("abort", () => context.teardown());
      return new Promise(() => {});
    });
    let refused = false;
    try { await leftover.callAsync("leftover"); } catch (error) { refused = /ended/.test(error.message); }
    results.push(refused && bgCounter(leftover, 4) === 0);
    for (const flag of [0, 16, 20]) {
      const guest = await load((context) => { if (!flag) context.teardown(); });
      if (flag) new DataView(guest.exports.memory.buffer).setUint32(flag, 1, true);
      let refused = false;
      try { if (flag) guest.call("buffers", ["first", "second"]); else guest.call("sync"); } catch (error) { refused = /ended/.test(error.message); }
      results.push(refused && bgCounter(guest, flag === 20 ? 24 : 12) === (flag === 20 ? 1 : 0));
    }
    return results;
  });
  expect(result).toEqual(Array(8).fill(true));
});

test("BG review: teardown reentry and callback errors cannot interrupt cleanup", async ({ page }) => {
  const result = await page.evaluate(async () => {
    const { ResourceHost } = await import("/bindings/javascript/ilang.mjs");
    const host = new ResourceHost();
    const context = host.context();
    let cleaned = 0;
    const id = context.insert("file", {}, () => { cleaned++; throw new Error("cleanup failed"); });
    context.insert("file", {}, () => { cleaned++; return Promise.reject(new Error("async cleanup failed")); });
    const lease = context.acquire(id, "file", "read");
    const provisional = context.provisional(id, "file");
    lease.signal.addEventListener("abort", () => { throw new Error("wake failed"); });
    let woke = false;
    lease.signal.onabort = () => { woke = true; };
    let reentered;
    const refused = [];
    context.ownRequest(() => {
      for (const action of [() => context.acquire(id, "file", "write"), () => lease.commit(42), () => provisional.deliver()]) {
        try { action(); refused.push(false); } catch (error) { refused.push(error.code === "closed"); }
      }
      reentered = context.teardown();
      throw new Error("cancel failed");
    });
    const ended = context.teardown();
    await ended;
    return { cleaned, woke, refused, joined: reentered === ended, errors: host.takeCleanupErrors().map((failure) => failure.error.message).sort(), drained: host.takeCleanupErrors().length };
  });
  expect(result).toEqual({ cleaned: 2, woke: true, refused: [true, true, true], joined: true, errors: ["async cleanup failed", "cancel failed", "cleanup failed", "wake failed"], drained: 0 });
});
