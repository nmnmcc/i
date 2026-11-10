import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { authorizeBindings, instantiate, readAbi, ResourceHost, GuestError, Stop } from "../ilang.mjs";
import { hostImports } from "../../../tests/browser/fixture-abi.mjs";

const bytes = await readFile(new URL("../../../tests/browser/fixtures/host-abi.wasm", import.meta.url));
const module = await WebAssembly.compile(bytes);
const abi = readAbi(module);
const digest = "sha256-" + [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))].map((b) => b.toString(16).padStart(2, "0")).join("");
function plan() {
  return { artifact_digest: digest, declarations: [ ...Object.entries(abi.services).flatMap(([namespace, fields]) => Object.entries(fields).map(([name, signature]) => ({ package: "trusted-fixture-content", module: "fixture/host", module_digest: digest, declaration: `service.${name}`, namespace, name, kind: "service", signature }))), ...Object.entries(abi.externals).flatMap(([namespace, fields]) => Object.entries(fields).map(([name, signature]) => ({ package: "trusted-fixture-content", module: "fixture/host", module_digest: digest, declaration: `external.${name}`, namespace, name, kind: "external", signature }))) ] };
}
async function authorized(contextImports, approve = () => true, declarations = plan()) {
  const bindings = await authorizeBindings(bytes, declarations, approve);
  const imports = hostImports(GuestError);
  for (const [namespace, fields] of Object.entries(contextImports)) for (const name of Object.keys(fields)) delete imports[namespace]?.[name];
  return { bindings, imports, contextImports };
}

test("native 3.1: every coalesced declaration needs its own grant", async () => {
  const manifest = plan();
  const original = manifest.declarations.find((d) => d.kind === "external");
  manifest.declarations.push({ ...original, declaration: "untrusted_alias" });
  const contextImports = { "app:math/v1": { hypot: (_, a, b) => Math.hypot(a, b) } };
  const options = await authorized(contextImports, (d) => d.declaration !== "untrusted_alias", manifest);
  await assert.rejects(instantiate(options.bindings.module, options), { message: "ungranted host import app:math/v1 hypot declared by fixture/host untrusted_alias" });
  const accepted = await authorized(contextImports, () => true, manifest);
  const guest = await instantiate(accepted.bindings.module, accepted);
  assert.equal(guest.call("hypot", [3, 4]), 5);
  manifest.declarations.push({ ...original });
  const twice = await authorized(contextImports, () => true, manifest);
  await instantiate(twice.bindings.module, twice);
});

test("native 3.1: artifact mismatch, missing declarations and self-asserted tokens fail", async () => {
  const changed = plan();
  changed.artifact_digest += "0";
  await assert.rejects(authorizeBindings(bytes, changed, () => true), /artifact digest/);
  const missing = plan();
  missing.declarations.pop();
  await assert.rejects(authorizeBindings(bytes, missing, () => true), /missing import/);
  const wrong = plan();
  wrong.declarations[0].signature = { params: [], result: "int", errors: [], async: false };
  await assert.rejects(authorizeBindings(bytes, wrong, () => true), /does not match ABI import/);
  await assert.rejects(instantiate(module, { imports: hostImports(GuestError), bindings: { module } }), /does not match/);
  await assert.rejects(authorizeBindings(module, plan(), () => true), /requires artifact bytes/);
  const approved = await authorizeBindings(bytes, plan(), () => true);
  await assert.rejects(instantiate(module, { imports: hostImports(GuestError), bindings: approved }), /does not match/);
  await assert.rejects(instantiate(bytes, { ...await authorized({}), bindings: undefined, imports: {}, contextImports: { "app:console/v1": { print: () => {} } } }), /ungranted host import/);
});

test("N9.1/N9.5: context-aware sync callbacks get fresh invocation tables and teardown", async () => {
  const contexts = [];
  let disposed = 0;
  const contextImports = { "app:console/v1": { print: (context) => {
    const id = context.insert("test", {}, () => { disposed++; });
    contexts.push({ context, id });
  } } };
  const options = { ...await authorized(contextImports), resourceHost: new ResourceHost() };
  for (let instance = 0; instance < 2; instance++) {
    const guest = await instantiate(options.bindings.module, options);
    guest.call("print", ["first"]);
    guest.call("print", ["second"]);
    guest.dispose();
  }
  assert.equal(contexts.length, 6);
  assert.equal(disposed, 6);
  for (const [i, { context, id }] of contexts.entries()) {
    assert.equal(context.live, false);
    if (i) assert.ok(id > contexts[i - 1].id);
  }
});

test("N9.5: async contexts teardown at stop, failure, disposal, and late replies stay in the old run", async () => {
  for (const mode of ["stop", "failure", "dispose"]) {
    const contexts = [];
    let disposed = 0;
    let late;
    const options = await authorized({ "app:clock/v1": { later: (context, value) => {
      context.insert("test", {}, () => { disposed++; });
      contexts.push(context);
      if (mode === "stop") return Promise.resolve(value);
      if (mode === "failure") return Promise.reject(new Error("host failed"));
      return new Promise((resolve) => { late = resolve; });
    } } });
    const guest = await instantiate(options.bindings.module, options);
    if (mode === "stop") await assert.rejects(guest.callAsync("app/async.stop_mid"), Stop);
    else if (mode === "failure") await assert.rejects(guest.callAsync("app/async.later", [1]), /host failed/);
    else {
      const pending = guest.callAsync("app/async.later", [1]);
      guest.dispose();
      await assert.rejects(pending, /disposed/);
      late(1);
      await Promise.resolve();
    }
    assert.ok(contexts.length > 0);
    assert.equal(disposed, contexts.length);
    assert.ok(contexts.every((context) => !context.live));
    for (const context of contexts) assert.throws(() => context.insert("test", {}, () => {}), /closed/);
  }
});

test("N9.1: two concurrent async runs in one host remain separate", async () => {
  const contexts = [];
  const resolve = [];
  const seen = new WeakSet();
  const options = { ...await authorized({ "app:clock/v1": { later: (context, value) => {
    if (seen.has(context)) return Promise.resolve(value);
    seen.add(context);
    const id = context.insert("test", value, () => {});
    contexts.push({ context, id });
    return new Promise((done) => { resolve.push(() => done(value)); });
  } } }), resourceHost: new ResourceHost() };
  const a = await instantiate(options.bindings.module, options);
  const b = await instantiate(options.bindings.module, options);
  const first = a.callAsync("app/async.later", [1]);
  const second = b.callAsync("app/async.later", [2]);
  assert.notEqual(contexts[0].id, contexts[1].id);
  assert.throws(() => contexts[0].context.acquire(contexts[1].id, "test", "read"), /closed/);
  resolve[0]();
  await first;
  assert.equal(contexts[0].context.live, false);
  assert.equal(contexts[1].context.live, true);
  resolve[1]();
  await second;
  assert.equal(contexts[1].context.live, false);
});

test("N9.3: close wakes an outstanding request and prevents late success", async () => {
  let captured;
  const options = await authorized({ "app:clock/v1": { later: (context) => {
    const id = context.insert("test", {}, () => {});
    const lease = context.acquire(id, "test", "read");
    captured = { context, id };
    return new Promise((resolve, reject) => {
      lease.signal.addEventListener("abort", () => {
        assert.throws(() => lease.commit(1), /closed/);
        reject(new GuestError("closed"));
      });
    });
  } } });
  const guest = await instantiate(options.bindings.module, options);
  const call = guest.callAsync("app/async.later", [1]);
  await captured.context.close(captured.id, "test");
  await assert.rejects(call, (error) => error instanceof GuestError && error.value === "closed");
});
