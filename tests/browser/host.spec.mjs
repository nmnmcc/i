import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/tests/browser/harness.html");
});

test("same ABI fixture supports GC, typed references and deep tail calls", async ({ page }) => {
  const result = await page.evaluate(async () => {
    const { instantiate, supportsRequiredFeatures } = await import("/bindings/javascript/ilang.mjs");
    const { fixtureAbi } = await import("/tests/browser/fixture-abi.mjs");
    const guest = await instantiate("/tests/browser/fixtures/host-abi.wasm", { abi: fixtureAbi });
    return { supported: supportsRequiredFeatures(), initialized: guest.exports.initialized(), typedCall: guest.exports.feature_probe(), tails: guest.exports.tail_count(1_000_000, 0), sum: guest.exports.gc_sum(100_000) };
  });
  expect(result).toEqual({ supported: true, initialized: 1, typedCall: 42, tails: 1_000_000, sum: 705_082_704 });
});

test("arbitrary precision values and UTF-8 survive copying and memory growth", async ({ page }) => {
  const result = await page.evaluate(async () => {
    const { instantiate } = await import("/bindings/javascript/ilang.mjs");
    const { fixtureAbi } = await import("/tests/browser/fixture-abi.mjs");
    const guest = await instantiate("/tests/browser/fixtures/host-abi.wasm", { abi: fixtureAbi });
    const numbers = [0n, 1n, -1n, 1n << 30n, -(1n << 64n), (1n << 4096n) + 0x123456789abcdefn];
    for (const number of numbers) if (guest.call("int_roundtrip", [number]) !== number) throw new Error("Integer lost precision");
    const strings = ["", "e\u0301 👩🏽‍🚀 中文\0", "界".repeat(65536)];
    for (const string of strings) if (guest.call("string_roundtrip", [string]) !== string) throw new Error("String changed");
    const bits = guest.call("bit_array_roundtrip", [{ bytes: Uint8Array.of(0xff, 0x80), bitLength: 9 }]);
    const empty = guest.call("bit_array_roundtrip", [{ bytes: new Uint8Array(), bitLength: 0 }]);
    return { allocations: guest.exports.live_allocations(), bytes: guest.exports.live_bytes(), bits: [...bits.bytes], bitLength: bits.bitLength, empty: empty.bitLength };
  });
  expect(result).toEqual({ allocations: 0, bytes: 0, bits: [255, 128], bitLength: 9, empty: 0 });
});

test("opaque references retain identity without inspecting guest GC objects", async ({ page }) => {
  const result = await page.evaluate(async () => {
    const { instantiate } = await import("/bindings/javascript/ilang.mjs");
    const { fixtureAbi } = await import("/tests/browser/fixture-abi.mjs");
    const guest = await instantiate("/tests/browser/fixtures/host-abi.wasm", { abi: fixtureAbi });
    const reference = { value: 42 };
    const same = guest.call("opaque_roundtrip", [reference]) === reference;
    return { same, null: guest.exports.reference_id(null) === 0n, stable: guest.exports.reference_id(reference) === guest.exports.reference_id(reference), distinct: guest.exports.reference_id(reference) !== guest.exports.reference_id({}), primitive: guest.exports.reference_id("same") === guest.exports.reference_id("same") };
  });
  expect(result).toEqual({ same: true, null: true, stable: true, distinct: true, primitive: true });
});

test("custom imports use ordinary host values with explicit ownership", async ({ page }) => {
  const result = await page.evaluate(async () => {
    const { instantiate } = await import("/bindings/javascript/ilang.mjs");
    const { customAbi, identityImports } = await import("/tests/browser/fixture-abi.mjs");
    const guest = await instantiate("/tests/browser/fixtures/custom-imports.wasm", { abi: customAbi, imports: identityImports() });
    const integer = -(1n << 1024n) + 19n;
    const string = "host 👩🏽‍🚀 value";
    const reference = {};
    return {
      integer: guest.call("echo_int", [integer]) === integer,
      string: guest.call("echo_string", [string]),
      bits: [...guest.call("echo_bits", [{ bytes: Uint8Array.of(0xc0), bitLength: 2 }]).bytes],
      reference: guest.call("echo_ref", [reference]) === reference,
      allocations: guest.exports.live_allocations(),
    };
  });
  expect(result).toEqual({ integer: true, string: "host 👩🏽‍🚀 value", bits: [192], reference: true, allocations: 0 });
});

test("ABI frames reclaim guest temporaries after host failures and support nested calls", async ({ page }) => {
  const result = await page.evaluate(async () => {
    const { instantiate } = await import("/bindings/javascript/ilang.mjs");
    const { customAbi, identityImports } = await import("/tests/browser/fixture-abi.mjs");
    const failure = new Error("original host failure");
    const throwing = await instantiate("/tests/browser/fixtures/custom-imports.wasm", { abi: customAbi, imports: identityImports({ echo_string() { throw failure; } }) });
    const initialHeap = throwing.exports.heap_end();
    for (let iteration = 0; iteration < 100; iteration++) {
      let caught;
      try { throwing.call("leaking_throw", ["temporary guest allocation"]); } catch (error) { caught = error; }
      if (caught !== failure) throw new Error("Primary error changed");
      if (throwing.exports.heap_end() !== initialHeap || throwing.exports.active_frames() !== 0 || throwing.exports.live_allocations() !== 0) throw new Error("Guest allocations leaked after a host exception");
    }
    let nested;
    nested = await instantiate("/tests/browser/fixtures/custom-imports.wasm", { abi: customAbi, imports: identityImports({ echo_string(text) {
      if (nested.exports.active_frames() !== 1) throw new Error("Outer frame missing");
      const number = -(1n << 256n);
      if (nested.call("echo_int", [number]) !== number || nested.exports.active_frames() !== 1) throw new Error("Nested frame corrupted outer call");
      return text;
    } }) });
    const nestedHeap = nested.exports.heap_end();
    const text = nested.call("leaking_throw", ["nested host call"]);
    return { text, heapRestored: nested.exports.heap_end() === nestedHeap, frames: nested.exports.active_frames(), allocations: nested.exports.live_allocations() };
  });
  expect(result).toEqual({ text: "nested host call", heapRestored: true, frames: 0, allocations: 0 });
});

test("panic reports source text, releases buffers and lets later tests run", async ({ page }) => {
  const result = await page.evaluate(async () => {
    const { instantiate, runTests } = await import("/bindings/javascript/ilang.mjs");
    const { fixtureAbi } = await import("/tests/browser/fixture-abi.mjs");
    const stdout = [];
    const guest = await instantiate("/tests/browser/fixtures/host-abi.wasm", { abi: fixtureAbi, stdout: (text) => stdout.push(text) });
    guest.run();
    let panic;
    try { guest.call("fail", ["module.i:7: panic detail"]); } catch (error) { panic = { name: error.name, message: error.message }; }
    const tests = await runTests("/tests/browser/fixtures/host-abi.wasm", { abi: fixtureAbi });
    return { stdout, panic, allocations: guest.exports.live_allocations(), tests: tests.map(({ passed }) => passed) };
  });
  expect(result).toEqual({ stdout: ["fixture main\n"], panic: { name: "iLangPanic", message: "module.i:7: panic detail" }, allocations: 0, tests: [true, false, true, true] });
});

test("random host callbacks support a memory-growing reentrant allocation", async ({ page }) => {
  const result = await page.evaluate(async () => {
    const { instantiate } = await import("/bindings/javascript/ilang.mjs");
    const { fixtureAbi } = await import("/tests/browser/fixture-abi.mjs");
    let guest;
    guest = await instantiate("/tests/browser/fixtures/host-abi.wasm", {
      abi: fixtureAbi,
      random(bytes) {
        const temporary = guest.exports.__ilang_alloc(131072);
        guest.exports.__ilang_free(temporary, 131072);
        bytes.fill(0xa5);
      },
    });
    const pointer = guest.exports.__ilang_alloc(8);
    const status = guest.exports.fill_random(pointer, 8);
    const bytes = [...new Uint8Array(guest.exports.memory.buffer, pointer, 8)];
    guest.exports.__ilang_free(pointer, 8);
    return { status, bytes, allocations: guest.exports.live_allocations() };
  });
  expect(result).toEqual({ status: 0, bytes: Array(8).fill(0xa5), allocations: 0 });
});

test("malformed entry and test metadata cannot turn non-Nil functions into passing tests", async ({ page }) => {
  const result = await page.evaluate(async () => {
    const { instantiate, runTests } = await import("/bindings/javascript/ilang.mjs");
    const { fixtureAbi } = await import("/tests/browser/fixture-abi.mjs");
    let entryError;
    try { await instantiate("/tests/browser/fixtures/host-abi.wasm", { abi: { ...fixtureAbi, entry: "feature_probe" } }); } catch (error) { entryError = error.message; }
    const tests = await runTests("/tests/browser/fixtures/host-abi.wasm", { abi: fixtureAbi, tests: ["feature_probe", "bool_roundtrip", "passing_test"] });
    return { entryError, tests: tests.map(({ passed }) => passed) };
  });
  expect(result).toEqual({ entryError: "Wasm signature does not match ABI descriptor for feature_probe", tests: [false, false, true] });
});

test("Wasm linking rejects same-arity scalar mismatches instead of coercing floats", async ({ page }) => {
  const result = await page.evaluate(async () => {
    const { instantiate } = await import("/bindings/javascript/ilang.mjs");
    const { fixtureAbi, customAbi, identityImports } = await import("/tests/browser/fixture-abi.mjs");
    const errors = [];
    try {
      await instantiate("/tests/browser/fixtures/host-abi.wasm", { abi: { ...fixtureAbi, exports: { bool_roundtrip: { params: ["float"], result: "float" } } } });
    } catch (error) { errors.push(error.name); }
    try {
      await instantiate("/tests/browser/fixtures/custom-imports.wasm", { abi: { ...customAbi, imports: { custom: { ...customAbi.imports.custom, echo_bool: { params: ["float"], result: "float" } } } }, imports: identityImports() });
    } catch (error) { errors.push(error.name); }
    return errors;
  });
  expect(result).toEqual(["iLangAbiError", "LinkError"]);
});
