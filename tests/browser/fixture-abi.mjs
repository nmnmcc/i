// Metadata and host functions shared by the Node.js and browser tests of the
// hand-written fixtures. host-abi.wasm has its own ilang.abi section.
const unary = (type) => ({ params: [type], result: type });
const allErrors = ["nil", "bool", "int", "float", "string", "bit_array"];
const fallible = { params: ["float"], result: "string", errors: allErrors };

export const customAbi = {
  version: 2,
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
export function customImports(iLangError, overrides = {}) {
  const custom = Object.fromEntries(Object.keys(customAbi.externals.custom).map((name) => [name, overrides[name] ?? ((value) => value)]));
  const errors = [undefined, false, 1n << 100n, -0.5, "oops", { bytes: Uint8Array.of(0xc0), bitLength: 2 }];
  return {
    custom,
    "custom:fallible/v1": {
      fallible: overrides.fallible ?? ((selector) => {
        if (selector === 0) return "fine";
        throw new iLangError(errors[Math.min(selector, 6) - 1]);
      }),
    },
  };
}

/** The host functions of host-abi.wasm; print writes to `printed`. */
export function hostImports(iLangError, printed = [], overrides = {}) {
  return {
    "app:console/v1": { print: (text) => { printed.push(text); } },
    "app:storage/v1": {
      read: overrides.read ?? ((key) => {
        if (key === "name") return "i";
        throw new iLangError(`no ${key}`);
      }),
    },
    "app:clock/v1": {
      later: overrides.later ?? (async (x) => {
        if (x < 0) throw new iLangError("negative");
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
export function compiledImports(iLangError, printed = [], overrides = {}) {
  const imports = hostImports(iLangError, printed, overrides);
  return {
    ...imports,
    "app:clock/v1": { ...imports["app:clock/v1"], tick: overrides.tick ?? (async (n) => n + 1n) },
  };
}
