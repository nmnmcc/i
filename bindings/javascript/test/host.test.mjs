import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { customAbi, customImports, fold, gateService, hostImports, nextMacrotask } from "../../../tests/browser/fixture-abi.mjs";
import { HOST_VALUE_REASON, instantiate, integerFromLimbs, integerToLimbs, readAbi, requestSignal, runTests, supportsRequiredFeatures, AbiError, GuestError, Stop } from "../ilang.mjs";

const fixture = await readFile(new URL("../../../tests/browser/fixtures/host-abi.wasm", import.meta.url));
const customFixture = await readFile(new URL("../../../tests/browser/fixtures/custom-imports.wasm", import.meta.url));
const noContinueFixture = await readFile(new URL("../../../tests/browser/fixtures/no-continue.wasm", import.meta.url));
const wrongTypesFixture = await readFile(new URL("../../../tests/browser/fixtures/wrong-types.wasm", import.meta.url));
const startRequestFixture = await readFile(new URL("../../../tests/browser/fixtures/start-request.wasm", import.meta.url));
const guest = (printed = [], overrides = {}) => instantiate(fixture, { imports: hostImports(GuestError, printed, overrides) });
const custom = (overrides = {}) => instantiate(customFixture, { abi: customAbi, imports: customImports(GuestError, overrides) });
const integers = [0n, 1n, -1n, (1n << 30n) - 1n, 1n << 30n, -(1n << 30n), 1n << 32n, -(1n << 64n), (1n << 4096n) + 0x123456789abcdefn];

function noAllocations(value) {
  assert.equal(value.exports.live_allocations(), 0);
  assert.equal(value.exports.live_bytes(), 0);
}

function stopsWith(message) {
  return (error) => error instanceof Stop && error.stopMessage === message;
}

test("engine supports GC, tail calls and exnref", () => {
  assert.equal(supportsRequiredFeatures(), true);
});

test("integer marshalling preserves arbitrary precision and rejects noncanonical values", () => {
  for (const value of integers) {
    const { sign, bytes } = integerToLimbs(value);
    assert.equal(integerFromLimbs(sign, bytes), value);
  }
  assert.throws(() => integerToLimbs(9007199254740993), TypeError);
  assert.throws(() => integerFromLimbs(2, new Uint8Array()), AbiError);
  assert.throws(() => integerFromLimbs(0, Uint8Array.of(1, 0, 0, 0)), AbiError);
  assert.throws(() => integerFromLimbs(1, Uint8Array.of(1)), AbiError);
  assert.throws(() => integerFromLimbs(-1, new Uint8Array(4)), AbiError);
});

test("metadata is read from the module", async () => {
  const value = await guest();
  assert.deepEqual(value.abi.entry, { name: "app/main.main", async: false });
  assert.equal(value.abi.tests.length, 6);
  assert.equal(value.abi.tests[5].async, true);
  assert.deepEqual(value.abi.exports.choose.errors, ["nil", "bool", "int", "float", "string", "bit_array"]);
  assert.equal(value.abi.exports["app/async.later"].async, true);
  assert.equal(value.abi.services["app:clock/v1"].later.async, true);
  assert.deepEqual(value.abi.externals["app:math/v1"].hypot, { params: ["float", "float"], result: "float", errors: [], async: false });
});

test("ilang_start runs once, before anything, and may call imports", async () => {
  const printed = [];
  const value = await guest(printed);
  assert.equal(value.exports.initialized(), 1);
  assert.deepEqual(printed, ["start\n"]);
  value.run();
  assert.deepEqual(printed, ["start\n", "fixture main\n"]);
  noAllocations(value);
});

test("values round trip and every buffer is freed", async () => {
  const value = await guest();
  for (const integer of integers) assert.equal(value.call("int_roundtrip", [integer]), integer);
  for (const text of ["", "hello\n", "a\0b", "é 👩🏽‍🚀 中文", "x".repeat(131072)]) {
    assert.equal(value.call("string_roundtrip", [text]), text);
  }
  for (const bits of [{ bytes: new Uint8Array(), bitLength: 0 }, { bytes: Uint8Array.of(0xa8), bitLength: 5 }, { bytes: Uint8Array.of(0xff, 0x80), bitLength: 9 }]) {
    assert.deepEqual(value.call("bit_array_roundtrip", [bits]), bits);
  }
  assert.equal(value.call("bool_roundtrip", [true]), true);
  assert.equal(value.call("bool_roundtrip", [false]), false);
  assert.equal(value.call("float_roundtrip", [-0]), -0);
  assert.equal(value.call("float_roundtrip", [1.2345]), 1.2345);
  assert.equal(value.call("nil_roundtrip"), undefined);
  const reference = { value: Symbol("opaque") };
  assert.equal(value.call("opaque_roundtrip", [reference]), reference);
  assert.equal(value.call("opaque_roundtrip", [null]), null);
  assert.equal(value.call("same_reference", [reference, reference]), true);
  assert.equal(value.call("same_reference", [reference, {}]), false);
  assert.equal(value.call("same_reference", [null, null]), true);
  assert.equal(value.call("exceptions"), 42);
  noAllocations(value);
});

test("errors use the tag and the canonical order", async () => {
  const value = await guest();
  assert.equal(value.call("choose", [0]), "ok");
  const expected = [undefined, true, -7n, 2.5, "bad", { bytes: Uint8Array.of(160), bitLength: 3 }];
  for (const [index, error] of expected.entries()) {
    assert.throws(() => value.call("choose", [index + 1]), (thrown) => {
      assert.ok(thrown instanceof GuestError);
      assert.deepEqual(thrown.value, error);
      return true;
    });
    noAllocations(value);
  }
  assert.throws(() => value.call("choose", [7]), /error tag 7/);
  assert.match(value.unusable, /error tag 7/);
  assert.throws(() => value.call("nil_roundtrip"), /make a new instance/);
});

test("host services and externals use the same encoding", async () => {
  const printed = [];
  const value = await guest(printed);
  assert.equal(value.call("read_through", ["name"]), "i");
  assert.throws(() => value.call("read_through", ["other"]), (error) => error instanceof GuestError && error.value === "no other");
  assert.equal(value.call("hypot", [3, 4]), 5);
  value.call("print", ["hello\n"]);
  value.call("print", [""]);
  assert.deepEqual(printed, ["start\n", "hello\n", ""]);
  noAllocations(value);
});

test("externals and every kind of service error round trip without leaks", async () => {
  const value = await custom();
  for (const integer of integers) assert.equal(value.call("echo_int", [integer]), integer);
  assert.equal(value.call("echo_string", ["🧪 custom import"]), "🧪 custom import");
  assert.equal(value.call("echo_string", [""]), "");
  assert.deepEqual(value.call("echo_bits", [{ bytes: Uint8Array.of(0xc0), bitLength: 2 }]), { bytes: Uint8Array.of(0xc0), bitLength: 2 });
  const reference = {};
  assert.equal(value.call("echo_ref", [reference]), reference);
  assert.equal(value.call("echo_bool", [true]), true);
  assert.equal(value.call("echo_float", [0.25]), 0.25);
  assert.equal(value.call("echo_nil"), undefined);
  assert.equal(value.call("fallible", [0]), "fine");
  const errors = [undefined, false, 1n << 100n, -0.5, "oops", { bytes: Uint8Array.of(0xc0), bitLength: 2 }];
  for (const [index, error] of errors.entries()) {
    assert.throws(() => value.call("fallible", [index + 1]), (thrown) => {
      assert.ok(thrown instanceof GuestError);
      assert.deepEqual(thrown.value, error);
      return true;
    });
  }
  noAllocations(value);
});

test("host values that are not correct stop the program", async () => {
  for (const [name, args] of [["float_roundtrip", [Infinity]], ["float_roundtrip", [NaN]], ["bool_roundtrip", [1]], ["string_roundtrip", ["\ud800"]], ["bit_array_roundtrip", [{ bytes: Uint8Array.of(1), bitLength: 1 }]], ["bit_array_roundtrip", [{ bytes: Uint8Array.of(0), bitLength: 9 }]], ["int_roundtrip", [1]]]) {
    const value = await guest();
    assert.throws(() => value.call(name, args), stopsWith(HOST_VALUE_REASON), name);
    assert.throws(() => value.call("nil_roundtrip"), /make a new instance/);
  }
  for (const read of [() => 1.5, () => { throw new GuestError(1n); }, () => { throw new GuestError(NaN); }]) {
    const value = await guest([], { read });
    assert.throws(() => value.call("read_through", ["key"]), stopsWith(HOST_VALUE_REASON));
    assert.ok(value.unusable);
  }
  const value = await guest();
  assert.throws(() => value.call("int_roundtrip", []), /expects 1/);
  assert.throws(() => value.call("absent", []), /descriptor/);
  assert.equal(value.unusable, null);
  noAllocations(value);
});

test("module values that are not correct are ABI errors", async () => {
  for (const name of ["bad_bool", "bad_utf8", "bad_tag"]) {
    const value = await guest();
    assert.throws(() => value.call(name), AbiError, name);
    assert.ok(value.unusable);
  }
});

test("a stop gives its message and the instance is unusable", async () => {
  const value = await guest();
  const message = "panic: boom\napp/users:12:5";
  assert.throws(() => value.call("stop", [message]), (error) => {
    assert.ok(error instanceof Stop);
    assert.equal(error.stopMessage, message);
    assert.ok(error.cause instanceof WebAssembly.RuntimeError);
    return true;
  });
  assert.throws(() => value.call("nil_roundtrip"), /make a new instance/);
  assert.throws(() => value.run(), /make a new instance/);
});

test("a failing host function keeps its error and the instance is unusable", async () => {
  const failure = new Error("the storage is gone");
  const value = await guest([], { read: () => { throw failure; } });
  assert.throws(() => value.call("read_through", ["key"]), (error) => error === failure);
  assert.ok(value.unusable);
  const throwing = await custom({ echo_string: () => { throw failure; } });
  assert.throws(() => throwing.call("echo_string", ["argument"]), (error) => error === failure);
  const promising = await custom({ echo_string: async (text) => text });
  assert.throws(() => promising.call("echo_string", ["argument"]), /must not give a Promise/);
  await assert.rejects(instantiate(customFixture, { abi: customAbi, imports: {} }), /Missing host function/);
});

test("an exception of a host function is not thrown into the module", async () => {
  const failure = new Error("the host is gone");
  for (const [name, args] of [["nil", []], ["float", [1.5]]]) {
    const value = await custom({ [`echo_${name}`]: () => { throw failure; } });
    // The handler of every exception around the import does not run: the
    // module traps, and the caller gets the exception.
    assert.throws(() => value.call(`guarded_${name}`, args), (error) => error === failure, name);
    assert.match(value.unusable, /the host is gone/);
    assert.throws(() => value.call("echo_nil"), /make a new instance/);
    const fine = await custom();
    assert.equal(fine.call(`guarded_${name}`, args), false);
  }
  // A value of another type, which the host cannot give to the module.
  const wrong = await custom({ echo_float: () => "text" });
  assert.throws(() => wrong.call("guarded_float", [1]), stopsWith(HOST_VALUE_REASON));
  assert.ok(wrong.unusable);
});

test("answers of the right type are given to the module unchecked by default", async () => {
  // The hand-written module gives the answer back as it is; a module built
  // by the compiler checks it (compiled.test.mjs).
  const value = await custom({ echo_float: () => NaN });
  assert.ok(Number.isNaN(value.exports.echo_float(1)));
  const checked = await instantiate(customFixture, { abi: customAbi, imports: customImports(GuestError, { echo_float: () => NaN }), checkReplies: true });
  assert.throws(() => checked.call("guarded_float", [1]), stopsWith(HOST_VALUE_REASON));
  assert.ok(checked.unusable);
});

test("tests run each in a new instance and a stop fails only its own test", async () => {
  const results = await runTests(fixture, { imports: hostImports(GuestError) });
  assert.deepEqual(results.map(({ name, passed }) => [name, passed]), [
    ["app/test.passing", true],
    ["app/test.failing", false],
    ["app/test.trapping", false],
    ["app/test.fresh", true],
    ["app/test.fresh", true],
    ["app/async_test.waits", true],
  ]);
  assert.ok(results[1].error instanceof Stop);
  assert.equal(results[1].error.stopMessage, "assertion failed: fixture\napp/test:3:5");
  assert.ok(results[2].error instanceof Stop);
  assert.equal(results[2].error.stopMessage, null);
});

test("async exports wait for host functions", async () => {
  const value = await guest();
  assert.equal(await value.callAsync("app/async.later", [1.5]), 2.5);
  await assert.rejects(value.callAsync("app/async.later", [-1]), (error) => error instanceof GuestError && error.value === "negative");
  noAllocations(value);
  assert.throws(() => value.call("app/async.later", [1]), /use callAsync/);
  await assert.rejects(value.callAsync("hypot", [1, 1]), /use call/);
  const waiting = value.callAsync("app/async.later", [1]);
  assert.throws(() => value.call("nil_roundtrip"), /waiting/);
  assert.equal(await waiting, 2);
  const failure = new Error("the clock is broken");
  const broken = await guest([], { later: async () => { throw failure; } });
  await assert.rejects(broken.callAsync("app/async.later", [1]), (error) => error === failure);
  assert.ok(broken.unusable);
});

test("GC, tail calls and exceptions remain available through the adapter", async () => {
  const value = await guest();
  assert.equal(value.exports.feature_probe(), 42);
  assert.equal(value.exports.tail_count(1_000_000, 0), 1_000_000);
  assert.equal(value.exports.gc_sum(10_000), 50_005_000);
});

test("metadata that is not correct fails before instantiation", async () => {
  const module = await WebAssembly.compile(customFixture);
  const base = { version: 4, entry: null, tests: [], exports: {}, services: {}, externals: {} };
  assert.equal(readAbi(module, base).entry, null);
  assert.equal(readAbi(module, base).version, 4);
  assert.equal(readAbi(module, { ...base, version: 3 }).version, 4);
  for (const old of [1, 2]) {
    assert.throws(() => readAbi(module, { ...base, version: old }), { message: `the module uses i ABI version ${old}; build it again with this version of i` });
  }
  assert.throws(() => readAbi(module, { ...base, version: base.version + 1 }), { message: `the module uses i ABI version ${base.version + 1}, but this host reads version ${base.version}; update the host` });
  for (const malformed of [
    null, [], { ...base, exports: [] }, { ...base, services: null }, { ...base, tests: null },
    { ...base, version: 1 }, { ...base, imports: {} }, { ...base, entry: "main" }, { version: base.version },
    { ...base, exports: { f: { params: ["list"], result: "nil" } } },
    { ...base, exports: { f: { params: [], result: "nil", errors: ["string", "int"] } } },
    { ...base, exports: { f: { params: [], result: "nil", errors: ["externref"] } } },
    { ...base, exports: { ilang_start: { params: [], result: "nil" } } },
    { ...base, exports: { ilang_continue: { params: [], result: "nil" } } },
    { ...base, entry: { name: "ilang_continue", async: true } },
    { ...base, entry: { name: "f" }, exports: { f: { params: [], result: "nil" } } },
    { ...base, services: { "ilang:host/v1": {} } },
    { ...base, externals: { m: { f: { params: [], result: "nil", errors: ["nil"] } } } },
  ]) {
    assert.throws(() => readAbi(module, malformed), AbiError, JSON.stringify(malformed));
  }
  const embedded = await WebAssembly.compile(fixture);
  assert.throws(() => readAbi(embedded, base), /cannot be overridden/);
});

test("Wasm linking rejects signatures that do not match the metadata", async () => {
  const wrongExport = { ...customAbi, exports: { ...customAbi.exports, echo_bool: { params: ["float"], result: "float" } } };
  await assert.rejects(instantiate(customFixture, { abi: wrongExport, imports: customImports(GuestError) }), /signature does not match.*echo_bool/);
  const wrongImport = { ...customAbi, externals: { custom: { ...customAbi.externals.custom, echo_bool: { params: ["float"], result: "float" } } } };
  await assert.rejects(instantiate(customFixture, { abi: wrongImport, imports: customImports(GuestError) }), WebAssembly.LinkError);
  const module = await WebAssembly.compile(customFixture);
  await assert.rejects(instantiate(module, { abi: wrongExport, imports: customImports(GuestError) }), /signature does not match/);
});

/** host-abi.wasm with gates for `later`. */
const gated = (options = {}) => {
  const gates = gateService({ requestSignal, ...options });
  return instantiate(fixture, { imports: hostImports(GuestError, [], { later: gates.later }) }).then((value) => ({ value, gates }));
};

test("many requests complete out of order", async () => {
  for (let seed = 0; seed < 200; seed++) {
    const n = 1 + (seed % 16);
    const { value, gates } = await gated({ seed });
    const h = await gates.drive(value.callAsync("app/async.many", [n]));
    assert.equal(h, fold(gates.answered()), `seed ${seed}`);
    assert.equal(gates.starts, 2 * n);
    assert.ok(gates.requests.every((gate) => !gate.signal.aborted));
    // The ids are used again in a second call.
    const again = await gates.drive(value.callAsync("app/async.many", [2]));
    assert.equal(again, fold(gates.answered().slice(2 * n)));
    assert.equal(value.unusable, null);
    noAllocations(value);
  }
});

test("the service runs during the import", async () => {
  const marks = [];
  const value = await guest([], { later: async (x) => { marks.push(x); return x; } });
  const call = value.callAsync("app/async.later", [1]);
  assert.deepEqual(marks, [1]);
  assert.equal(await call, 2);
  assert.deepEqual(marks, [1, 2]);
});

test("a service may not call the instance during the import", async () => {
  for (const reenter of [(value) => value.call("nil_roundtrip"), (value) => value.callAsync("app/async.later", [2])]) {
    let value;
    let starts = 0;
    value = await guest([], {
      later: (x) => {
        starts++;
        const inner = reenter(value);
        return inner instanceof Promise ? inner.then(() => x) : x;
      },
    });
    await assert.rejects(value.callAsync("app/async.later", [1]), { message: "The instance is running Wasm; a host function may not call it" });
    assert.equal(starts, 1);
    assert.ok(value.unusable);
    assert.throws(() => value.call("nil_roundtrip"), /make a new instance/);
  }
});

test("the calling convention of services is unchanged, and requestSignal works only while a service starts", async () => {
  const value = await guest([], { later: (x, scale = 2) => x * scale });
  assert.equal(await value.callAsync("app/async.later", [1.5]), 8);
  assert.throws(() => requestSignal(), { message: "requestSignal() may only be called while an async service starts" });
  const late = await guest([], { later: async (x) => { await null; requestSignal(); return x; } });
  await assert.rejects(late.callAsync("app/async.later", [1]), { message: "requestSignal() may only be called while an async service starts" });
  assert.ok(late.unusable);
});

test("a stop drops outstanding requests and aborts their signals", async () => {
  const { value, gates } = await gated({ seed: 7 });
  await assert.rejects(gates.drive(value.callAsync("app/async.stop_mid")), (error) => {
    assert.ok(error instanceof Stop);
    assert.equal(error.stopMessage, "fixture: stop");
    assert.equal(gates.requests.filter((gate) => gate.signal.aborted).length, 2);
    return true;
  });
  assert.equal(gates.starts, 3);
  assert.throws(() => value.call("nil_roundtrip"), /make a new instance/);
});

test("a host failure drops the others", async () => {
  const { value, gates } = await gated({ seed: 3, fail: 2 });
  await assert.rejects(gates.drive(value.callAsync("app/async.fan", [5])), (error) => {
    assert.equal(error.message, "the gate failed");
    assert.ok(gates.requests.filter((gate) => gate.x !== 2).every((gate) => gate.signal.aborted || gates.released.includes(gate)));
    return true;
  });
  assert.equal(value.unusable, "the gate failed");
});

test("status 1 with nothing outstanding and other statuses are faults", async () => {
  for (const [name, message] of [
    ["app/async.stuck", "the module waits, but no host function is running"],
    ["app/async.bad_status", "export app/async.bad_status gave the status 7, which is not 0, 1 or 2"],
  ]) {
    const { value } = await gated();
    await assert.rejects(value.callAsync(name), { message });
    assert.equal(value.unusable, message);
  }
});

test("duplicate and negative ids trap before the service runs", async () => {
  for (const [name, message, starts] of [
    ["app/async.dup", "the module started request 7 twice", 1],
    ["app/async.negative", "the module started request -1, which is below 0", 0],
  ]) {
    const { value, gates } = await gated();
    await assert.rejects(value.callAsync(name), { message });
    assert.equal(value.unusable, message);
    assert.equal(gates.starts, starts);
  }
});

test("requests start and are cancelled only during an async call", async () => {
  for (const [name, message, starts] of [
    ["app/sync.starts", "the module started request 3 outside an async call", 0],
    ["app/sync.cancels", "the module cancelled request 0 outside an async call", 0],
    ["app/async.nested", "the module started request 77 outside an async call", 0],
    ["app/async.nested_cancel", "the module cancelled request 5 outside an async call", 1],
    ["app/async.result_starts", "the module started request 6 outside an async call", 1],
    ["app/async.result_cancels", "the module cancelled request 0 outside an async call", 1],
  ]) {
    const { value, gates } = await gated();
    if (name.startsWith("app/sync.")) assert.throws(() => value.call(name), { message }, name);
    else await assert.rejects(gates.drive(value.callAsync(name)), { message }, name);
    assert.equal(value.unusable, message, name);
    assert.equal(gates.starts, starts, name);
  }
});

test("an abort listener cannot start a call while the requests are dropped", async () => {
  const baseline = await guest([], { later: async (x) => 2 * x });
  const expected = await baseline.callAsync("app/async.leftover");
  const reentered = [];
  let holding = true;
  let value;
  value = await guest([], {
    later: (x) => {
      if (!holding || x === 1) return Promise.resolve(2 * x);
      requestSignal().addEventListener("abort", () => {
        reentered.push(value.callAsync("app/async.later", [1]).then(() => "ran", (error) => error.message));
      });
      return new Promise(() => {});
    },
  });
  assert.equal(await value.callAsync("app/async.leftover"), expected);
  holding = false;
  assert.deepEqual(await Promise.all(reentered), [
    "An async call is waiting; only its host functions may answer now",
    "An async call is waiting; only its host functions may answer now",
  ]);
  assert.equal(value.unusable, null);
  assert.equal(await value.callAsync("app/async.later", [1]), await baseline.callAsync("app/async.later", [1]));
});

test("a request started by ilang_start fails the instantiation", async () => {
  let starts = 0;
  const later = async (x) => {
    starts++;
    return x;
  };
  await assert.rejects(instantiate(startRequestFixture, { imports: { "app:clock/v1": { later } } }), {
    message: "the module started request 0 outside an async call",
  });
  assert.equal(starts, 0);
});

test("ilang_continue and ilang_resume exports of the wrong type are faults", async () => {
  for (const [name, message, aborted] of [
    ["app/async.runnable", "the module is runnable, but has no ilang_continue", 0],
    ["app/async.bad_resume", "the module does not export ilang_resume:app:clock/v1:later", 1],
  ]) {
    const signals = [];
    const later = async (x) => {
      signals.push(requestSignal());
      return x;
    };
    const value = await instantiate(wrongTypesFixture, { imports: { "app:clock/v1": { later, ready: async () => {} } } });
    await assert.rejects(value.callAsync(name), { message }, name);
    assert.equal(value.unusable, message, name);
    assert.throws(() => value.call("app/sync.nil"), /make a new instance/, name);
    assert.equal(value.exports.calls(), 0, name);
    assert.equal(signals.filter((signal) => signal.aborted).length, aborted, name);
  }
});

test("status 2 waits reuse one MessageChannel where there is no setImmediate", async () => {
  const saved = { setImmediate: globalThis.setImmediate, MessageChannel: globalThis.MessageChannel };
  let channels = 0;
  let messages = 0;
  let channel;
  let reply;
  const delivered = () => {
    // Hold the answer until at least two calls of ilang_continue have run.
    if (++messages === 3) reply();
  };
  globalThis.MessageChannel = class extends saved.MessageChannel {
    constructor() {
      super();
      channels++;
      channel = this;
      this.port1.addEventListener("message", delivered);
    }
  };
  delete globalThis.setImmediate;
  let timeout;
  try {
    const value = await guest([], { later: (x) => new Promise((resolve) => { reply = () => resolve(2 * x); }) });
    // Keep Node alive while the SDK's channel is unreferenced, and bound a hang.
    const deadline = new Promise((_, reject) => {
      timeout = setTimeout(() => reject(new Error("MessageChannel waits did not finish")), 5000);
    });
    const turns = await Promise.race([value.callAsync("app/async.yielding", [1e7]), deadline]);
    assert.ok(turns >= 2, `${turns}`);
    assert.equal(channels, 1, `${channels} channels for ${turns} turns`);
    assert.ok(messages >= 3, `${messages} messages`);
  } finally {
    clearTimeout(timeout);
    channel?.port1.removeEventListener("message", delivered);
    globalThis.setImmediate = saved.setImmediate;
    globalThis.MessageChannel = saved.MessageChannel;
  }
});

test("leftovers are dropped at status 0", async () => {
  const { value, gates } = await gated({ seed: 5 });
  const h = await gates.drive(value.callAsync("app/async.leftover").then((result) => {
    assert.equal(gates.requests.filter((gate) => gate.signal.aborted).length, 2);
    return result;
  }));
  assert.equal(h, fold(gates.answered()));
  assert.equal(gates.answered().length, 1);
  assert.equal(value.unusable, null);
  const many = await gates.drive(value.callAsync("app/async.many", [3]));
  assert.equal(many, fold(gates.answered().slice(1)));
});

test("cancel drops one request and aborts its signal", async () => {
  for (let seed = 0; seed < 200; seed++) {
    const n = 2 + (seed % 15);
    const { value, gates } = await gated({ seed });
    const h = await gates.drive(value.callAsync("app/async.cancel", [n]));
    assert.equal(h, fold(gates.answered()), `seed ${seed}`);
    const cancelled = gates.requests.filter((gate) => gate.signal.aborted);
    assert.equal(cancelled.length, Math.floor(n / 2));
    assert.ok(cancelled.every((gate) => !gates.released.includes(gate)));
  }
  const { value } = await gated();
  await assert.rejects(value.callAsync("app/async.cancel_bad"), { message: "the module cancelled request 12345, which is not outstanding" });
  assert.ok(value.unusable);
});

test("status 2 gives ready answers and calls ilang_continue", async () => {
  const value = await guest([], { later: async (x) => 2 * x });
  const count = await value.callAsync("app/async.yielding", [1000]);
  assert.ok(count >= 1 && count < 1000, `${count}`);
  const timer = await guest([], { later: (x) => new Promise((resolve) => setTimeout(() => resolve(2 * x), 1)) });
  const waited = await timer.callAsync("app/async.yielding", [1e7]);
  assert.ok(waited >= 1 && waited < 1e7, `${waited}`);
});

test("every exit of the wait drops the requests", async () => {
  const { value, gates } = await gated({ seed: 2, wrongTypeFirst: true });
  await assert.rejects(gates.drive(value.callAsync("app/async.fan", [3])), (error) => {
    assert.ok(error instanceof Stop);
    assert.equal(error.stopMessage, HOST_VALUE_REASON);
    assert.equal(gates.requests.filter((gate) => gate.signal.aborted).length, 2);
    return true;
  });
  assert.equal(value.unusable, HOST_VALUE_REASON);
  // Late answers of the dropped requests change nothing.
  for (const gate of gates.requests) gate.resolve(2 * gate.x);
  await nextMacrotask();
  assert.equal(value.unusable, HOST_VALUE_REASON);
});

test("cancel and reuse an id whose answer is finished", async () => {
  let finished = 0;
  for (let seed = 0; seed < 200; seed++) {
    const n = 2 + (seed % 15);
    const { value, gates } = await gated({ seed, burst: true });
    const h = await gates.drive(value.callAsync("app/async.cancel_reuse", [n]));
    assert.equal(h, fold(gates.answered()), `seed ${seed}`);
    finished += gates.released.filter((gate) => gate.signal.aborted).length;
  }
  assert.ok(finished > 0);
});

test("requests start and are cancelled from ilang_continue", async () => {
  for (let seed = 0; seed < 200; seed++) {
    const n = 2 + (seed % 8);
    const { value, gates } = await gated({ seed });
    const h = await gates.drive(value.callAsync("app/async.continue_io", [n]));
    assert.equal(h, fold(gates.answered()), `seed ${seed}`);
    assert.equal(gates.starts, n);
    const held = gates.requests.filter((gate) => gate.x === 0);
    assert.equal(held.length, 1);
    assert.ok(held[0].signal.aborted);
  }
});

test("a module without ilang_continue or an ilang_resume export breaks", async () => {
  for (const [name, message] of [
    ["app/async.runnable", "the module is runnable, but has no ilang_continue"],
    ["app/async.no_resume", "the module does not export ilang_resume:app:clock/v1:later"],
  ]) {
    const value = await instantiate(noContinueFixture, { imports: { "app:clock/v1": { later: async (x) => x, ready: async () => {} } } });
    await assert.rejects(value.callAsync(name), { message });
    assert.equal(value.unusable, message);
    assert.throws(() => value.call("app/sync.nil"), /make a new instance/);
    assert.equal(value.exports.calls(), 0);
  }
  const value = await guest([], { later: () => new Promise(() => {}) });
  void value.callAsync("app/async.later", [1]);
  assert.throws(() => value.exports.ilang_continue(), WebAssembly.RuntimeError);
  // No answer may resume this instance after the raw trap.
});

// A module must have the export that answers a request when a request starts,
// or the export `ilang_continue` when status 2 is returned,
// even when an answer is ready, the entry ends, or the request is cancelled.
test("required exports are checked before delivery or drop", async () => {
  for (const [fixture, badResume] of [[noContinueFixture, "no_resume"], [wrongTypesFixture, "bad_resume"]]) {
    for (const suffix of ["runnable_ready", `${badResume}_done`, `${badResume}_cancel`]) {
      for (const ready of [false, true]) {
        const signals = [];
        const work = () => {
          signals.push(requestSignal());
          return ready ? Promise.resolve() : new Promise(() => {});
        };
        const value = await instantiate(fixture, { imports: { "app:clock/v1": { later: work, ready: work } } });
        const message = suffix === "runnable_ready" ? "the module is runnable, but has no ilang_continue"
          : "the module does not export ilang_resume:app:clock/v1:later";
        await assert.rejects(value.callAsync(`app/async.${suffix}`), { message });
        assert.equal(value.unusable, message);
        assert.throws(() => value.call("app/sync.nil"), /make a new instance/);
        assert.equal(value.exports.calls(), 0);
        assert.equal(signals.length, 1);
        assert.ok(signals.every((signal) => signal.aborted));
        await nextMacrotask();
        assert.equal(value.unusable, message);
      }
    }
  }
});
