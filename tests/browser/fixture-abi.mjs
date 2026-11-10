// Metadata and host functions shared by the Node.js and browser tests of the
// hand-written fixtures. host-abi.wasm has its own ilang.abi section.
const unary = (type) => ({ params: [type], result: type });
const allErrors = ["nil", "bool", "int", "float", "string", "bit_array"];
const fallible = { params: ["float"], result: "string", errors: allErrors };

export const customAbi = {
  version: 3,
  entry: null,
  tests: [],
  exports: {
    echo_string: unary("string"),
    echo_int: unary("int"),
    echo_bits: unary("bit_array"),
    echo_ref: unary("externref"),
    echo_bool: unary("bool"),
    echo_float: unary("float"),
    echo_nil: { params: [], result: "nil" },
    fallible,
    guarded_nil: { params: [], result: "bool" },
    guarded_float: { params: ["float"], result: "bool" },
  },
  services: { "custom:fallible/v1": { fallible } },
  externals: {
    custom: {
      echo_string: unary("string"),
      echo_int: unary("int"),
      echo_bits: unary("bit_array"),
      echo_ref: unary("externref"),
      echo_bool: unary("bool"),
      echo_float: unary("float"),
      echo_nil: { params: [], result: "nil" },
    },
  },
};

/** The externals echo their argument; fallible gives each kind of error. */
export function customImports(GuestError, overrides = {}) {
  const custom = Object.fromEntries(Object.keys(customAbi.externals.custom).map((name) => [name, overrides[name] ?? ((value) => value)]));
  const errors = [undefined, false, 1n << 100n, -0.5, "oops", { bytes: Uint8Array.of(0xc0), bitLength: 2 }];
  return {
    custom,
    "custom:fallible/v1": {
      fallible: overrides.fallible ?? ((selector) => {
        if (selector === 0) return "fine";
        throw new GuestError(errors[Math.min(selector, 6) - 1]);
      }),
    },
  };
}

/** The host functions of host-abi.wasm; print writes to `printed`. */
export function hostImports(GuestError, printed = [], overrides = {}) {
  return {
    "app:console/v1": { print: (text) => { printed.push(text); } },
    "app:storage/v1": {
      read: overrides.read ?? ((key) => {
        if (key === "name") return "i";
        throw new GuestError(`no ${key}`);
      }),
    },
    "app:clock/v1": {
      later: overrides.later ?? (async (x) => {
        if (x < 0) throw new GuestError("negative");
        return x;
      }),
    },
    "app:math/v1": { hypot: (a, b) => Math.hypot(a, b) },
  };
}

/**
 * The host functions of compiled.wasm, built by the compiler from
 * tests/browser/fixtures/compiled: those of host-abi.wasm, and the async
 * `tick` of `app:clock/v1`, which gives n + 1.
 */
export function compiledImports(GuestError, printed = [], overrides = {}) {
  const imports = hostImports(GuestError, printed, overrides);
  return {
    ...imports,
    "app:clock/v1": { ...imports["app:clock/v1"], tick: overrides.tick ?? (async (n) => n + 1n) },
    ...opaqueImports(overrides.opaque ?? null),
  };
}

/** One macrotask: setImmediate in Node.js, a MessageChannel message in browsers. */
export function nextMacrotask() {
  if (typeof setImmediate === "function") return new Promise((resolve) => setImmediate(resolve));
  return new Promise((resolve) => {
    const channel = new MessageChannel();
    channel.port1.onmessage = () => {
      channel.port1.close();
      resolve();
    };
    channel.port2.postMessage(null);
  });
}

/** The hash that host-abi.wasm folds over the arguments of its answers: h = (h * 31 + x) mod (2^31 - 1). */
export function fold(xs) {
  let h = 0n;
  for (const x of xs) h = (h * 31n + BigInt(x)) % 2147483647n;
  return Number(h);
}

/**
 * A service for `later` of host-abi.wasm whose answers come in an order that
 * a seeded random choice makes, with no timers. Each request is a gate that
 * answers 2x once it is released. `drive(promise)` releases one gate at a
 * time (two with `burst`), chosen among the gates that are live then, with a
 * macrotask between, until the promise settles. A gate whose argument is 0
 * is held: it is never released.
 */
export function gateService({ seed = 0, burst = false, requestSignal = null, fail = null, wrongTypeFirst = false } = {}) {
  let state = Math.imul(seed + 1, 2654435761) >>> 0 || 1;
  const random = () => {
    state ^= state << 13;
    state >>>= 0;
    state ^= state >>> 17;
    state ^= state << 5;
    state >>>= 0;
    return state;
  };
  const gates = { live: [], released: [], requests: [], starts: 0 };
  gates.later = (x) => {
    gates.starts++;
    const signal = requestSignal ? requestSignal() : null;
    return new Promise((resolve, reject) => {
      const gate = { x, signal, resolve, reject };
      gates.requests.push(gate);
      if (x !== 0) gates.live.push(gate);
      // A dropped request is never released.
      signal?.addEventListener("abort", () => {
        const index = gates.live.indexOf(gate);
        if (index >= 0) gates.live.splice(index, 1);
      });
    });
  };
  const release = () => {
    const count = Math.min(burst ? 2 : 1, gates.live.length);
    for (let index = 0; index < count; index++) {
      const [gate] = gates.live.splice(random() % gates.live.length, 1);
      gates.released.push(gate);
      if (gate.x === fail) gate.reject(new Error("the gate failed"));
      else if (wrongTypeFirst && gates.released.length === 1) gate.resolve("wrong");
      else gate.resolve(2 * gate.x);
    }
  };
  gates.drive = async (promise) => {
    let done = false;
    promise.then(() => { done = true; }, () => { done = true; });
    for (let round = 0; !done; round++) {
      if (round > 100000) throw new Error("the gates did not finish");
      await nextMacrotask();
      if (!done) release();
    }
    return promise;
  };
  /** The arguments of the released gates whose requests were answered, in order. */
  gates.answered = () => gates.released.filter((gate) => !gate.signal?.aborted).map((gate) => gate.x);
  return gates;
}

/** The same raw value enters each declared opaque type (spec 4.1, 11.4). */
export function opaqueImports(reference) {
  return {
    "app:opaque/v1": { get: () => reference, echo: (value) => value },
    "app:opaque-sync/v1": { get_a: () => reference, get_b: () => reference },
    "app:opaque-async/v1": { get_a: async () => reference, get_b: async () => reference },
  };
}

/** Run the compiled nominal-identity matrix in Node.js and browsers. */
export async function checkOpaque(guest, reference, other) {
  const call = (name, args = []) => guest.call(`app/opaque.${name}`, args);
  const results = [];
  for (const [a, b, same] of [[reference, reference, true], [reference, other, false], [null, null, true], [null, reference, false]]) {
    for (const name of ["direct_aa", "generic_aa", "alias_aa", "collection_aa", "roundtrip_aa"]) {
      results.push(call(name, [a, b]) === same);
    }
    // Different constructors differ even when the foreign identity is shared.
    for (const name of ["direct_ab", "generic_ab", "module_ab", "collection_ab"]) {
      results.push(call(name, [a, b]) === false);
    }
  }
  for (const a of [reference, null]) {
    const returned = call("echo", [a]);
    results.push(returned === a, call("generic_aa", [a, returned]) === true);
    results.push(call("generic_ab", [a, returned]) === false);
    results.push(call("roundtrip_ab", [a]) === false);
    results.push(call("hash_aa", [a, a]) === true);
    results.push(call("inspect_a", [a]) === "//wasm(opaque)");
  }
  for (const name of ["import_aa", "sync_aa"]) results.push(call(name) === true);
  for (const name of ["import_ab", "sync_ab"]) results.push(call(name) === false);
  results.push(await guest.callAsync("app/opaque.async_aa") === true);
  results.push(await guest.callAsync("app/opaque.async_ab") === false);
  return results.every(Boolean);
}

/** Counters in bg-lifecycle.wat prove that no guest call occurs after end. */
export const bgLifecycleAbi = {
  version: 3,
  entry: { name: "entry", async: true },
  tests: [],
  exports: {
    runnable: { params: [], result: "nil", errors: [], async: true },
    immediate: { params: [], result: "nil", errors: [], async: true },
    leftover: { params: [], result: "nil", errors: [], async: true },
    sync: { params: [], result: "nil", errors: [], async: false },
    bits: { params: ["bit_array"], result: "bit_array", errors: [], async: false },
    buffers: { params: ["string", "string"], result: "nil", errors: [], async: false },
  },
  services: {
    "bg:control/v1": {
      capture: { params: [], result: "nil", errors: [], async: false },
      end: { params: [], result: "nil", errors: [], async: false },
    },
    "bg:work/v1": { wait: { params: [], result: "nil", errors: [], async: true } },
  },
  externals: {},
};
