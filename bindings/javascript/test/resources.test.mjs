import test from "node:test";
import assert from "node:assert/strict";
import { ResourceHost, ResourceError } from "../resources.mjs";

const code = (expected) => (error) => error instanceof ResourceError && error.code === expected;

test("N9.1/N9.2: contexts share monotone BigInt identities, never resource tables", async () => {
  const host = new ResourceHost();
  let disposed = 0;
  const a = host.context();
  const id = a.insert("file", {}, () => { disposed++; });
  assert.equal(typeof id, "bigint");
  await a.close(id, "file");
  const b = host.context();
  const next = b.insert("file", {}, () => { disposed++; });
  assert.equal(next, id + 1n);
  assert.throws(() => a.acquire(next, "file", "cursor"), code("closed"));
  assert.throws(() => b.acquire(id, "file", "cursor"), code("closed"));
  await Promise.all([a.teardown(), b.teardown()]);
  assert.equal(disposed, 2);
});

test("N9.2: exhaustion disposes failed creations and cannot recycle", async () => {
  const context = new ResourceHost({ limit: 1n }).context();
  let disposed = 0;
  const create = () => context.insert("file", {}, () => { disposed++; });
  const id = create();
  await context.close(id, "file");
  assert.throws(create, code("memory exhausted"));
  assert.throws(create, code("memory exhausted"));
  assert.equal(disposed, 3);
});

test("N9.1/N9.3: unknown numbers, wrong kinds, leases, completion and stale numbers", async () => {
  const context = new ResourceHost().context();
  const value = {};
  const id = context.insert("stream", value, () => {});
  for (const number of [0n, -1n, 999n, 1, "1"]) {
    assert.throws(() => context.acquire(number, "stream", "read"), code("closed"));
    await context.close(number, "stream");
  }
  assert.throws(() => context.acquire(id, "file", "read"), code("closed"));
  const read = context.acquire(id, "stream", "read");
  assert.equal(read.value, value);
  assert.throws(() => context.acquire(id, "stream", "read"), code("busy"));
  assert.equal(context.acquire(id, "stream", "write").commit(42), 42);
  await context.close(id, "file");
  assert.equal(read.signal.aborted, false);
  read.release();
  const next = context.acquire(id, "stream", "read");
  await context.close(id, "stream");
  assert.equal(next.signal.aborted, true);
  assert.throws(() => next.commit(1), code("closed"));
  assert.throws(() => context.acquire(id, "stream", "read"), code("closed"));
});

test("N9.3/N9.5: closes join cleanup, reentrant close joins, teardown includes closing jobs", async () => {
  const context = new ResourceHost().context();
  let finish;
  let cleanups = 0;
  const id = context.insert("child", {}, () => {
    cleanups++;
    return new Promise((resolve) => { finish = resolve; });
  });
  const lease = context.acquire(id, "child", "wait");
  let reentrant;
  lease.signal.addEventListener("abort", () => { reentrant = context.close(id, "child"); });
  const a = context.close(id, "child");
  const b = context.close(id, "child");
  assert.equal(a, b);
  assert.equal(reentrant, a);
  let ended = false;
  const end = context.teardown().then(() => { ended = true; });
  await Promise.resolve();
  assert.equal(ended, false);
  assert.equal(cleanups, 1);
  finish();
  await end;
  await b;
  assert.equal(ended, true);
  assert.notEqual(context.close(id, "child"), a); // Completed receipt was freed.
});

test("N9.4/N9.5: provisional disposal, delivered ownership, late creation and idempotent teardown", async () => {
  const host = new ResourceHost();
  const context = host.context();
  let disposed = 0;
  const create = () => context.insert("file", {}, () => { disposed++; });
  const first = context.provisional(create(), "file");
  await first.discard();
  const second = context.provisional(create(), "file");
  second.deliver();
  await second.discard();
  assert.equal(disposed, 1);
  await context.teardown();
  await context.teardown();
  assert.equal(disposed, 2);
  assert.throws(create, code("closed"));
  assert.equal(disposed, 3);
});

test("N9.4/N9.5: teardown joins a creation job before its resource exists", async () => {
  const context = new ResourceHost().context();
  const job = context.cleanupJob();
  let ended = false;
  const end = context.teardown().then(() => { ended = true; });
  await Promise.resolve();
  assert.equal(ended, false);
  assert.throws(() => context.cleanupJob(), /closed/);
  let cleaned = false;
  assert.throws(() => context.insert("late", {}, () => { cleaned = true; job.complete(); }), /closed/);
  await end;
  assert.equal(cleaned, true);
  assert.equal(ended, true);
});
