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
    const { instantiate, supportsRequiredFeatures, hostImports, GuestError } = await eval(loader)();
    const printed = [];
    const guest = await instantiate("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(GuestError, printed) });
    return { supported: supportsRequiredFeatures(), printed, initialized: guest.exports.initialized(), typedCall: guest.exports.feature_probe(), tails: guest.exports.tail_count(1_000_000, 0), sum: guest.exports.gc_sum(100_000), exceptions: guest.call("exceptions") };
  }, `(${load})`);
  expect(result).toEqual({ supported: true, printed: ["start\n"], initialized: 1, typedCall: 42, tails: 1_000_000, sum: 705_082_704, exceptions: 42 });
});

test("arbitrary precision values and UTF-8 survive copying and memory growth", async ({ page }) => {
  const result = await page.evaluate(async (loader) => {
    const { instantiate, hostImports, GuestError } = await eval(loader)();
    const guest = await instantiate("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(GuestError) });
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
    const { instantiate, hostImports, customAbi, customImports, GuestError } = await eval(loader)();
    const guest = await instantiate("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(GuestError) });
    const custom = await instantiate("/tests/browser/fixtures/custom-imports.wasm", { abi: customAbi, imports: customImports(GuestError) });
    const errors = (value, name) => [1, 2, 3, 4, 5, 6].map((selector) => {
      try { value.call(name, [selector]); return "no error"; } catch (error) {
        const thrown = error.value;
        return error instanceof GuestError ? (thrown && thrown.bytes ? [...thrown.bytes] : typeof thrown === "bigint" ? `${thrown}n` : thrown ?? "nil") : error.message;
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
    const { instantiate, hostImports, GuestError } = await eval(loader)();
    const guest = await instantiate("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(GuestError) });
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
    const { instantiate, runTests, hostImports, GuestError, HOST_VALUE_REASON } = await eval(loader)();
    const printed = [];
    const guest = await instantiate("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(GuestError, printed) });
    guest.run();
    let stop;
    try { guest.call("stop", ["panic: boom\napp/users:12:5"]); } catch (error) { stop = { name: error.name, message: error.stopMessage }; }
    let refused;
    try { guest.call("nil_roundtrip"); } catch (error) { refused = error.name; }
    const incorrect = await instantiate("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(GuestError) });
    let host;
    try { incorrect.call("float_roundtrip", [NaN]); } catch (error) { host = error.stopMessage === HOST_VALUE_REASON; }
    const tests = await runTests("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(GuestError) });
    return { printed, stop, refused, host, tests: tests.map(({ passed }) => passed), message: tests[1].error.stopMessage };
  }, `(${load})`);
  expect(result).toEqual({
    printed: ["start\n", "fixture main\n"],
    stop: { name: "Stop", message: "panic: boom\napp/users:12:5" },
    refused: "AbiError",
    host: true,
    tests: [true, false, false, true, true, true],
    message: "assertion failed: fixture\napp/test:3:5",
  });
});

test("async exports wait for host functions", async ({ page }) => {
  const result = await page.evaluate(async (loader) => {
    const { instantiate, hostImports, GuestError } = await eval(loader)();
    const guest = await instantiate("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(GuestError) });
    const value = await guest.callAsync("app/async.later", [1.5]);
    let error;
    try { await guest.callAsync("app/async.later", [-1]); } catch (thrown) { error = thrown.value; }
    return { value, error, allocations: guest.exports.live_allocations() };
  }, `(${load})`);
  expect(result).toEqual({ value: 2.5, error: "negative", allocations: 0 });
});

test("a module built by the compiler runs with sync and async host services", async ({ page }) => {
  const result = await page.evaluate(async (loader) => {
    const { instantiate, compiledImports, GuestError } = await eval(loader)();
    const printed = [];
    const guest = await instantiate("/tests/browser/fixtures/compiled.wasm", { imports: compiledImports(GuestError, printed) });
    const size = guest.call("app/app.greet", ["i"]);
    let error;
    try { guest.call("app/app.lookup", ["other"]); } catch (thrown) { error = thrown.value; }
    const sum = await guest.callAsync("app/app.wait_sum", [1.5, 2]);
    let negative;
    try { await guest.callAsync("app/app.wait_sum", [1, -1]); } catch (thrown) { negative = thrown.value; }
    await guest.run();
    return { size: String(size), error, distance: guest.call("app/app.distance", [3, 4]), sum, negative, printed, unusable: guest.unusable };
  }, `(${load})`);
  expect(result).toEqual({ size: "1", error: "no other", distance: 5, sum: 3.5, negative: "negative", printed: ["hello i", "main 42 1.5"], unusable: null });
});

test("Wasm linking rejects signatures that do not match the metadata", async ({ page }) => {
  const result = await page.evaluate(async (loader) => {
    const { instantiate, customAbi, customImports, GuestError } = await eval(loader)();
    const errors = [];
    try {
      await instantiate("/tests/browser/fixtures/custom-imports.wasm", { abi: { ...customAbi, exports: { ...customAbi.exports, echo_bool: { params: ["float"], result: "float" } } }, imports: customImports(GuestError) });
    } catch (error) { errors.push(error.name); }
    try {
      await instantiate("/tests/browser/fixtures/custom-imports.wasm", { abi: { ...customAbi, externals: { custom: { ...customAbi.externals.custom, echo_bool: { params: ["float"], result: "float" } } } }, imports: customImports(GuestError) });
    } catch (error) { errors.push(error.name); }
    return errors;
  }, `(${load})`);
  expect(result).toEqual(["AbiError", "LinkError"]);
});

test("host functions never throw into the module, and the module checks their answers", async ({ page }) => {
  const result = await page.evaluate(async (loader) => {
    const { instantiate, customAbi, customImports, compiledImports, GuestError } = await eval(loader)();
    const failure = new Error("the host is gone");
    const guarded = {};
    for (const [name, args] of [["nil", []], ["float", [1.5]]]) {
      const custom = await instantiate("/tests/browser/fixtures/custom-imports.wasm", { abi: customAbi, imports: customImports(GuestError, { [`echo_${name}`]: () => { throw failure; } }) });
      try { guarded[name] = custom.call(`guarded_${name}`, args); } catch (error) { guarded[name] = error === failure ? "thrown to the caller" : String(error); }
    }
    const compiled = await instantiate("/tests/browser/fixtures/compiled.wasm", { imports: { ...compiledImports(GuestError), "app:math/v1": { hypot: () => NaN } } });
    let stop;
    try { compiled.call("app/app.distance", [3, 4]); } catch (error) { stop = error.stopMessage; }
    return { guarded, stop };
  }, `(${load})`);
  expect(result).toEqual({
    guarded: { nil: "thrown to the caller", float: "thrown to the caller" },
    stop: "host: value is not correct\napp/app:48:35",
  });
});

test("many requests complete out of order", async ({ page }) => {
  const result = await page.evaluate(async (loader) => {
    const { instantiate, hostImports, gateService, fold, requestSignal, GuestError } = await eval(loader)();
    const module = await WebAssembly.compile(await (await fetch("/tests/browser/fixtures/host-abi.wasm")).arrayBuffer());
    const wrong = [];
    for (let seed = 0; seed < 200; seed++) {
      const n = 1 + (seed % 16);
      const gates = gateService({ seed, requestSignal });
      const guest = await instantiate(module, { imports: hostImports(GuestError, [], { later: gates.later }) });
      const h = await gates.drive(guest.callAsync("app/async.many", [n]));
      if (h !== fold(gates.answered()) || gates.starts !== 2 * n || guest.unusable !== null) wrong.push(seed);
    }
    return wrong;
  }, `(${load})`);
  expect(result).toEqual([]);
});

test("status 2 and cancel", async ({ page }) => {
  const result = await page.evaluate(async (loader) => {
    const { instantiate, hostImports, gateService, fold, requestSignal, GuestError } = await eval(loader)();
    // A service that is ready at once: the answer comes after status 2,
    // and ilang_continue ends the call.
    const ready = await instantiate("/tests/browser/fixtures/host-abi.wasm", { imports: hostImports(GuestError, [], { later: async (x) => 2 * x }) });
    const count = await ready.callAsync("app/async.yielding", [1000]);
    const module = await WebAssembly.compile(await (await fetch("/tests/browser/fixtures/host-abi.wasm")).arrayBuffer());
    const wrong = [];
    for (let seed = 0; seed < 50; seed++) {
      for (const [name, n, burst] of [["app/async.continue_io", 2 + (seed % 8), false], ["app/async.cancel_reuse", 2 + (seed % 15), true]]) {
        const gates = gateService({ seed, burst, requestSignal });
        const guest = await instantiate(module, { imports: hostImports(GuestError, [], { later: gates.later }) });
        const h = await gates.drive(guest.callAsync(name, [n]));
        if (h !== fold(gates.answered()) || guest.unusable !== null) wrong.push([name, seed]);
      }
    }
    return { yielded: count >= 1 && count < 1000, wrong };
  }, `(${load})`);
  expect(result).toEqual({ yielded: true, wrong: [] });
});

// Exercise production compiler output, including coalesced external imports.
test("compiled opaque boundaries preserve nominal identity", async ({ page }) => {
  const result = await page.evaluate(async (loader) => {
    const { instantiate, compiledImports, checkOpaque, GuestError } = await eval(loader)();
    const reference = {};
    for (const opaque of [reference, null]) {
      const guest = await instantiate("/tests/browser/fixtures/compiled.wasm", {
        imports: compiledImports(GuestError, [], { opaque }),
      });
      if (!await checkOpaque(guest, reference, {})) return false;
    }
    return true;
  }, `(${load})`);
  expect(result).toBe(true);
});

test("ABI 5 Int boundaries cross compiled imports, exports and async replies", async ({ page }) => {
  await page.goto("/tests/browser/harness.html");
  const count = await page.evaluate(async () => {
    const { instantiate, GuestError } = await import("/bindings/javascript/ilang.mjs");
    const { compiledImports, checkIntegers } = await import("/tests/browser/fixture-abi.mjs");
    const guest = await instantiate("/tests/browser/fixtures/compiled.wasm", {
      imports: compiledImports(GuestError, [], { tick: async n => n }),
    });
    return checkIntegers(guest, GuestError);
  });
  expect(count).toBe(13);
});
