import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { fixtureAbi, customAbi, identityImports } from "../../../tests/browser/fixture-abi.mjs";
import { instantiate, integerFromLimbs, integerToLimbs, readAbi, runTests, supportsRequiredFeatures, iLangAbiError, iLangPanic } from "../ilang.mjs";

const fixture = await readFile(new URL("../../../tests/browser/fixtures/host-abi.wasm", import.meta.url));
const customFixture = await readFile(new URL("../../../tests/browser/fixtures/custom-imports.wasm", import.meta.url));
const guest = (options = {}) => instantiate(fixture, { abi: fixtureAbi, ...options });
const integers = [0n, 1n, -1n, (1n << 30n) - 1n, 1n << 30n, -(1n << 30n), 1n << 32n, -(1n << 64n), (1n << 4096n) + 0x123456789abcdefn];

function noAllocations(value) {
  assert.equal(value.exports.live_allocations(), 0);
  assert.equal(value.exports.live_bytes(), 0);
  assert.equal(value.exports.active_frames(), 0);
}

test("engine supports all three required Wasm features", () => {
  assert.equal(supportsRequiredFeatures(), true);
});

test("integer marshalling preserves arbitrary precision and rejects noncanonical values", () => {
  for (const value of integers) {
    const { sign, bytes } = integerToLimbs(value);
    assert.equal(integerFromLimbs(sign, bytes), value);
  }
  assert.throws(() => integerToLimbs(9007199254740993), TypeError);
  assert.throws(() => integerFromLimbs(2, new Uint8Array()), iLangAbiError);
  assert.throws(() => integerFromLimbs(0, Uint8Array.of(1, 0, 0, 0)), iLangAbiError);
  assert.throws(() => integerFromLimbs(1, Uint8Array.of(1)), iLangAbiError);
  assert.throws(() => integerFromLimbs(-1, new Uint8Array(4)), iLangAbiError);
});

test("primitive round trips release inputs and copied results, including empty buffers", async () => {
  const value = await guest();
  assert.equal(value.exports.initialized(), 1);
  for (const integer of integers) assert.equal(value.call("int_roundtrip", [integer]), integer);
  for (const text of ["", "hello\n", "a\0b", "e\u0301 👩🏽‍🚀 中文", "x".repeat(131072)]) {
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
  noAllocations(value);
});

test("invalid API values are rejected before entering Wasm", async () => {
  const value = await guest();
  assert.throws(() => value.call("float_roundtrip", [Infinity]), /finite/);
  assert.throws(() => value.call("float_roundtrip", [NaN]), /finite/);
  assert.throws(() => value.call("bool_roundtrip", [1]), /booleans/);
  assert.throws(() => value.call("string_roundtrip", ["\ud800"]), /surrogate/);
  assert.throws(() => value.call("bit_array_roundtrip", [{ bytes: Uint8Array.of(1), bitLength: 1 }]), /Unused/);
  assert.throws(() => value.call("bit_array_roundtrip", [{ bytes: Uint8Array.of(0), bitLength: 9 }]), /byte length/);
  assert.throws(() => value.call("int_roundtrip", []), /expects 1/);
  assert.throws(() => value.call("absent", []), /descriptor/);
  noAllocations(value);
});

test("host output is exact, random providers fill guest buffers and errors carry messages", async () => {
  const output = [];
  const errors = [];
  const value = await guest({ stdout: (text) => output.push(text), stderr: (text) => errors.push(text), random: (bytes) => bytes.fill(0xab) });
  value.run();
  assert.deepEqual(output, ["fixture main\n"]);
  assert.deepEqual(errors, []);
  const pointer = value.exports.__ilang_alloc(8);
  assert.equal(value.exports.fill_random(pointer, 8), 0);
  assert.deepEqual(new Uint8Array(value.exports.memory.buffer, pointer, 8), new Uint8Array(8).fill(0xab));
  value.exports.__ilang_free(pointer, 8);
  assert.throws(() => value.call("fail", ["source.i:7: panic detail"]), (error) => error instanceof iLangPanic && error.message === "source.i:7: panic detail");
  assert.throws(() => value.call("trap_with_string", ["freed even when trapping"]), WebAssembly.RuntimeError);
  assert.throws(() => value.exports.emit(3, 0, 0), /channel 3/);
  assert.throws(() => value.exports.fill_random(0x7fffffff, 32), /outside linear memory/);
  noAllocations(value);
});

test("opaque custom imports and allocated return values round trip without leakage", async () => {
  const value = await instantiate(customFixture, { abi: customAbi, imports: identityImports() });
  for (const integer of integers) assert.equal(value.call("echo_int", [integer]), integer);
  assert.equal(value.call("echo_string", ["🧪 custom import"]), "🧪 custom import");
  assert.deepEqual(value.call("echo_bits", [{ bytes: Uint8Array.of(0xc0), bitLength: 2 }]), { bytes: Uint8Array.of(0xc0), bitLength: 2 });
  const reference = {};
  assert.equal(value.call("echo_ref", [reference]), reference);
  assert.equal(value.call("echo_bool", [true]), true);
  assert.equal(value.call("echo_float", [0.25]), 0.25);
  assert.equal(value.call("echo_nil"), undefined);
  noAllocations(value);
});

test("custom host failures and asynchronous callbacks preserve ownership", async () => {
  const throwing = await instantiate(customFixture, { abi: customAbi, imports: identityImports({ echo_string: () => { throw new Error("host failure"); } }) });
  assert.throws(() => throwing.call("echo_string", ["argument"]), /host failure/);
  noAllocations(throwing);
  const asynchronous = await instantiate(customFixture, { abi: customAbi, imports: identityImports({ echo_string: async (text) => text }) });
  assert.throws(() => asynchronous.call("echo_string", ["argument"]), /synchronous/);
  noAllocations(asynchronous);
  await assert.rejects(instantiate(customFixture, { abi: customAbi, imports: {} }), /Missing host function/);
});

test("ABI frames reclaim guest temporaries after repeated host failures and nest across reentry", async () => {
  const failure = new Error("original host failure");
  const throwing = await instantiate(customFixture, { abi: customAbi, imports: identityImports({ echo_string: () => { throw failure; } }) });
  const initialHeap = throwing.exports.heap_end();
  for (let iteration = 0; iteration < 100; iteration++) {
    assert.throws(() => throwing.call("leaking_throw", ["guest allocates before callback"]), (error) => error === failure);
    assert.equal(throwing.exports.heap_end(), initialHeap);
    noAllocations(throwing);
  }
  let nested;
  nested = await instantiate(customFixture, { abi: customAbi, imports: identityImports({ echo_string(text) {
    assert.equal(nested.exports.active_frames(), 1);
    const number = -(1n << 256n);
    assert.equal(nested.call("echo_int", [number]), number);
    assert.equal(nested.exports.active_frames(), 1);
    return text;
  } }) });
  const nestedHeap = nested.exports.heap_end();
  assert.equal(nested.call("leaking_throw", ["nested host call"]), "nested host call");
  assert.equal(nested.exports.heap_end(), nestedHeap);
  noAllocations(nested);
});

test("test instances are isolated and one panic does not suppress later tests", async () => {
  const results = await runTests(fixture, { abi: fixtureAbi });
  assert.deepEqual(results.map(({ passed }) => passed), [true, false, true, true]);
  assert.ok(results[1].error instanceof iLangPanic);
});

test("GC and tail calls remain available through the adapter", async () => {
  const value = await guest();
  assert.equal(value.exports.feature_probe(), 42);
  assert.equal(value.exports.tail_count(1_000_000, 0), 1_000_000);
  assert.equal(value.exports.gc_sum(10_000), 50_005_000);
});

test("host reference identities remain stable within each instance", async () => {
  const value = await guest();
  const first = {};
  const second = {};
  assert.equal(value.exports.reference_id(null), 0n);
  assert.equal(value.exports.reference_id(first), value.exports.reference_id(first));
  assert.notEqual(value.exports.reference_id(first), value.exports.reference_id(second));
  assert.equal(value.exports.reference_id("primitive"), value.exports.reference_id("primitive"));
  assert.notEqual(value.exports.reference_id(undefined), 0n);
});

test("invalid ABI versions and signatures fail before instantiation", async () => {
  const module = await WebAssembly.compile(fixture);
  for (const malformed of [null, [], { version: 1, exports: [] }, { version: 1, imports: null }, { version: 1, tests: null }]) {
    assert.throws(() => readAbi(module, malformed), iLangAbiError);
  }
  assert.throws(() => readAbi(module, { version: 2 }), /version 2/);
  assert.throws(() => readAbi(module, { version: 1, exports: { bad: { params: ["list"], result: "nil" } } }), /Unknown ABI type/);
  assert.throws(() => readAbi(module, { version: 1, imports: { "ilang:host/v1": {} } }), /reserved/);
  assert.equal(readAbi(module, { version: 1, entry: null }).entry, undefined);
});

test("entry, test and typed export signatures must match their ABI", async () => {
  await assert.rejects(guest({ abi: { ...fixtureAbi, entry: "feature_probe" } }), /signature does not match/);
  await assert.rejects(guest({ abi: { ...fixtureAbi, exports: { bool_roundtrip: { params: [], result: "bool" } } } }), /signature does not match/);
  const results = await runTests(fixture, { abi: fixtureAbi, tests: ["feature_probe", "bool_roundtrip", "passing_test"] });
  assert.deepEqual(results.map(({ passed }) => passed), [false, false, true]);
  assert.match(results[0].error.message, /signature does not match/);
  assert.match(results[1].error.message, /signature does not match/);
});

test("Wasm linking rejects same-arity scalar type mismatches in exports and imports", async () => {
  const abi = { ...fixtureAbi, exports: { bool_roundtrip: { params: ["float"], result: "float" } } };
  await assert.rejects(guest({ abi }), /signature does not match.*bool_roundtrip/);
  const wrongImport = { ...customAbi, imports: { custom: { ...customAbi.imports.custom, echo_bool: { params: ["float"], result: "float" } } } };
  await assert.rejects(instantiate(customFixture, { abi: wrongImport, imports: identityImports() }), WebAssembly.LinkError);
  // Precompiled modules must receive the same strict checks without source bytes.
  const module = await WebAssembly.compile(fixture);
  await assert.rejects(instantiate(module, { abi }), /signature does not match/);
});
