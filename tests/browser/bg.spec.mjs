import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => { await page.goto("/tests/browser/harness.html"); });

// Expectations: native N9.1–N9.3 (kind/context checks, never reuse, close order).
test("BG resources isolate runs, revoke leases and join cleanup", async ({ page }) => {
  const result = await page.evaluate(async () => {
    const { ResourceHost } = await import("/bindings/javascript/ilang.mjs");
    const host = new ResourceHost();
    const a = host.context();
    const b = host.context();
    let finish;
    let count = 0;
    const id = a.insert("test", {}, () => { count++; return new Promise((resolve) => { finish = resolve; }); });
    const next = b.insert("test", {}, () => {});
    const lease = a.acquire(id, "test", "read");
    const errors = [];
    for (const [context, kind] of [[b, "test"], [a, "other"]]) {
      try { context.acquire(id, kind, "read"); } catch (error) { errors.push(error.code); }
    }
    try { a.acquire(id, "test", "read"); } catch (error) { errors.push(error.code); }
    const receipt = a.close(id, "test");
    const same = receipt === a.close(id, "test");
    try { lease.commit(1); } catch (error) { errors.push(error.code); }
    let ended = false;
    const end = a.teardown().then(() => { ended = true; });
    await Promise.resolve();
    const waited = !ended;
    finish();
    await end;
    const unaffected = b.live;
    await b.teardown();
    return { errors, same, waited, unaffected, count, monotone: next > id, revoked: lease.signal.aborted };
  });
  expect(result).toEqual({ errors: ["closed", "closed", "busy", "closed"], same: true, waited: true, unaffected: true, count: 1, monotone: true, revoked: true });
});

// Native 3.1: a grant belongs to a declaration, including aliases coalesced to
// one wire name; artifact bytes and explicit host approval are both required.
test("BG validates declaration grants before linking and tears down invocations", async ({ page }) => {
  const result = await page.evaluate(async () => {
    const { authorizeBindings, readAbi, instantiate, GuestError } = await import("/bindings/javascript/ilang.mjs");
    const { hostImports } = await import("/tests/browser/fixture-abi.mjs");
    const bytes = await (await fetch("/tests/browser/fixtures/host-abi.wasm")).arrayBuffer();
    const module = await WebAssembly.compile(bytes);
    const abi = readAbi(module);
    const digest = "sha256-" + [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))].map((b) => b.toString(16).padStart(2, "0")).join("");
    const declarations = [];
    for (const [kind, table] of [["service", abi.services], ["external", abi.externals]]) {
      for (const [namespace, fields] of Object.entries(table)) {
        for (const [name, signature] of Object.entries(fields)) declarations.push({ package: "fixture-content", module: "fixture/host", module_digest: digest, declaration: `${kind}.${name}`, namespace, name, kind, signature });
      }
    }
    declarations.push({ ...declarations.find((d) => d.name === "hypot"), declaration: "alias" });
    const plan = { artifact_digest: digest, declarations };
    const imports = hostImports(GuestError);
    delete imports["app:math/v1"].hypot;
    let disposed = 0;
    const contextImports = { "app:math/v1": { hypot: (context, a, b) => {
      context.insert("test", {}, () => { disposed++; });
      return Math.hypot(a, b);
    } } };
    const refused = await authorizeBindings(bytes, plan, (d) => d.declaration !== "alias");
    let message;
    try { await instantiate(refused.module, { imports, contextImports, bindings: refused }); } catch (error) { message = error.message; }
    const bindings = await authorizeBindings(bytes, plan, () => true);
    const guest = await instantiate(bindings.module, { imports, contextImports, bindings });
    const values = [guest.call("hypot", [3, 4]), guest.call("hypot", [5, 12])];
    guest.dispose();
    return { message, values, disposed };
  });
  expect(result).toEqual({ message: "ungranted host import app:math/v1 hypot declared by fixture/host alias", values: [5, 13], disposed: 2 });
});
