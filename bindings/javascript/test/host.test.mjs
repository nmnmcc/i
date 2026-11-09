import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { customAbi, customImports, hostImports } from "../../../tests/browser/fixture-abi.mjs";
import { HOST_VALUE_REASON, instantiate, integerFromLimbs, integerToLimbs, readAbi, runTests, supportsRequiredFeatures, AbiError, GuestError, Stop } from "../ilang.mjs";

const fixture = await readFile(new URL("../../../tests/browser/fixtures/host-abi.wasm", import.meta.url));
const customFixture = await readFile(new URL("../../../tests/browser/fixtures/custom-imports.wasm", import.meta.url));
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

test("metadata that does not follow B.6 fails before instantiation", async () => {
  const module = await WebAssembly.compile(customFixture);
  const base = { version: 2, entry: null, tests: [], exports: {}, services: {}, externals: {} };
  assert.equal(readAbi(module, base).entry, null);
  for (const malformed of [
    null, [], { ...base, exports: [] }, { ...base, services: null }, { ...base, tests: null },
    { ...base, version: 1 }, { ...base, imports: {} }, { ...base, entry: "main" }, { version: 2 },
    { ...base, exports: { f: { params: ["list"], result: "nil" } } },
    { ...base, exports: { f: { params: [], result: "nil", errors: ["string", "int"] } } },
    { ...base, exports: { f: { params: [], result: "nil", errors: ["externref"] } } },
    { ...base, exports: { ilang_start: { params: [], result: "nil" } } },
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
