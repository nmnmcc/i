import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.goto("/tests/browser/harness.html");
});

const load = async () => {
  const sdk = await import("/bindings/javascript/ilang.mjs");
  const fixtures = await import("/tests/browser/fixture-abi.mjs");
  return { ...sdk, ...fixtures };
};

test("the engine has GC, typed references, tail calls and exnref", async ({ page }) => {
  const result = await page.evaluate(async (loader) => {
    const { instantiate, supportsRequiredFeatures, hostImports, iLangError } = await eval(loader)();
    const printed = [];
    const guest = await instantiate("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(iLangError, printed) });
    return { supported: supportsRequiredFeatures(), printed, initialized: guest.exports.initialized(), typedCall: guest.exports.feature_probe(), tails: guest.exports.tail_count(1_000_000, 0), sum: guest.exports.gc_sum(100_000), exceptions: guest.call("exceptions") };
  }, `(${load})`);
  expect(result).toEqual({ supported: true, printed: ["start\n"], initialized: 1, typedCall: 42, tails: 1_000_000, sum: 705_082_704, exceptions: 42 });
});

test("arbitrary precision values and UTF-8 survive copying and memory growth", async ({ page }) => {
  const result = await page.evaluate(async (loader) => {
    const { instantiate, hostImports, iLangError } = await eval(loader)();
    const guest = await instantiate("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(iLangError) });
    const numbers = [0n, 1n, -1n, 1n << 30n, -(1n << 64n), (1n << 4096n) + 0x123456789abcdefn];
    for (const number of numbers) if (guest.call("int_roundtrip", [number]) !== number) throw new Error("Integer lost precision");
    const strings = ["", "é 👩🏽‍🚀 中文\0", "界".repeat(65536)];
    for (const string of strings) if (guest.call("string_roundtrip", [string]) !== string) throw new Error("String changed");
    const bits = guest.call("bit_array_roundtrip", [{ bytes: Uint8Array.of(0xff, 0x80), bitLength: 9 }]);
    const empty = guest.call("bit_array_roundtrip", [{ bytes: new Uint8Array(), bitLength: 0 }]);
    return { allocations: guest.exports.live_allocations(), bytes: guest.exports.live_bytes(), bits: [...bits.bytes], bitLength: bits.bitLength, empty: empty.bitLength };
  }, `(${load})`);
  expect(result).toEqual({ allocations: 0, bytes: 0, bits: [255, 128], bitLength: 9, empty: 0 });
});

test("errors carry their tag and every member of the error type", async ({ page }) => {
  const result = await page.evaluate(async (loader) => {
    const { instantiate, hostImports, customAbi, customImports, iLangError } = await eval(loader)();
    const guest = await instantiate("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(iLangError) });
    const custom = await instantiate("/tests/browser/fixtures/custom-imports.wasm", { abi: customAbi, imports: customImports(iLangError) });
    const errors = (value, name) => [1, 2, 3, 4, 5, 6].map((selector) => {
      try { value.call(name, [selector]); return "no error"; } catch (error) {
        const thrown = error.value;
        return error instanceof iLangError ? (thrown && thrown.bytes ? [...thrown.bytes] : typeof thrown === "bigint" ? `${thrown}n` : thrown ?? "nil") : error.message;
      }
    });
    return {
      ok: [guest.call("choose", [0]), custom.call("fallible", [0])],
      guest: errors(guest, "choose"),
      custom: errors(custom, "fallible"),
      read: guest.call("read_through", ["name"]),
      allocations: guest.exports.live_allocations() + custom.exports.live_allocations(),
    };
  }, `(${load})`);
  expect(result).toEqual({
    ok: ["ok", "fine"],
    guest: ["nil", true, "-7n", 2.5, "bad", [160]],
    custom: ["nil", false, "1267650600228229401496703205376n", -0.5, "oops", [192]],
    read: "i",
    allocations: 0,
  });
});

test("opaque references keep identity through reference_id", async ({ page }) => {
  const result = await page.evaluate(async (loader) => {
    const { instantiate, hostImports, iLangError } = await eval(loader)();
    const guest = await instantiate("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(iLangError) });
    const reference = { value: 42 };
    return {
      same: guest.call("opaque_roundtrip", [reference]) === reference,
      identical: guest.call("same_reference", [reference, reference]),
      distinct: guest.call("same_reference", [reference, {}]),
      nulls: guest.call("same_reference", [null, null]),
    };
  }, `(${load})`);
  expect(result).toEqual({ same: true, identical: true, distinct: false, nulls: true });
});

test("a stop reports its message, makes the instance unusable, and fails only its own test", async ({ page }) => {
  const result = await page.evaluate(async (loader) => {
    const { instantiate, runTests, hostImports, iLangError, HOST_VALUE_REASON } = await eval(loader)();
    const printed = [];
    const guest = await instantiate("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(iLangError, printed) });
    guest.run();
    let stop;
    try { guest.call("stop", ["panic: boom\napp/users:12:5"]); } catch (error) { stop = { name: error.name, message: error.stopMessage }; }
    let refused;
    try { guest.call("nil_roundtrip"); } catch (error) { refused = error.name; }
    const incorrect = await instantiate("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(iLangError) });
    let host;
    try { incorrect.call("float_roundtrip", [NaN]); } catch (error) { host = error.stopMessage === HOST_VALUE_REASON; }
    const tests = await runTests("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(iLangError) });
    return { printed, stop, refused, host, tests: tests.map(({ passed }) => passed), message: tests[1].error.stopMessage };
  }, `(${load})`);
  expect(result).toEqual({
    printed: ["start\n", "fixture main\n"],
    stop: { name: "iLangStop", message: "panic: boom\napp/users:12:5" },
    refused: "iLangAbiError",
    host: true,
    tests: [true, false, false, true, true, true],
    message: "assertion failed: fixture\napp/test:3:5",
  });
});

test("async exports wait for host functions", async ({ page }) => {
  const result = await page.evaluate(async (loader) => {
    const { instantiate, hostImports, iLangError } = await eval(loader)();
    const guest = await instantiate("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(iLangError) });
    const value = await guest.callAsync("app/async.later", [1.5]);
    let error;
    try { await guest.callAsync("app/async.later", [-1]); } catch (thrown) { error = thrown.value; }
    return { value, error, allocations: guest.exports.live_allocations() };
  }, `(${load})`);
  expect(result).toEqual({ value: 2.5, error: "negative", allocations: 0 });
});

test("Wasm linking rejects signatures that do not match the metadata", async ({ page }) => {
  const result = await page.evaluate(async (loader) => {
    const { instantiate, customAbi, customImports, iLangError } = await eval(loader)();
    const errors = [];
    try {
      await instantiate("/tests/browser/fixtures/custom-imports.wasm", { abi: { ...customAbi, exports: { ...customAbi.exports, echo_bool: { params: ["float"], result: "float" } } }, imports: customImports(iLangError) });
    } catch (error) { errors.push(error.name); }
    try {
      await instantiate("/tests/browser/fixtures/custom-imports.wasm", { abi: { ...customAbi, externals: { custom: { ...customAbi.externals.custom, echo_bool: { params: ["float"], result: "float" } } } }, imports: customImports(iLangError) });
    } catch (error) { errors.push(error.name); }
    return errors;
  }, `(${load})`);
  expect(result).toEqual(["iLangAbiError", "LinkError"]);
});
