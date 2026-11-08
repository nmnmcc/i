/**
 * iLang host ABI v1. Language values live in the guest; this module only loads
 * modules and copies values across the foreign-function boundary.
 * SPDX-License-Identifier: Apache-2.0
 */

const encoder = new TextEncoder();
const decoder = new TextDecoder("utf-8", { fatal: true });
const kinds = new Set(["nil", "bool", "float", "int", "string", "bit_array", "externref"]);
const namespace = "ilang:host/v1";
const arities = { nil: 0, bool: 1, float: 1, int: 3, string: 2, bit_array: 2, externref: 1 };
const wasmTypes = { nil: [], bool: [0x7f], float: [0x7c], int: [0x7f, 0x7f, 0x7f], string: [0x7f, 0x7f], bit_array: [0x7f, 0x7f], externref: [0x6f] };
const signatureModules = new Map();
const checkedSignatures = new WeakMap();
const embeddedAbis = new WeakMap();
// Generated from tests/browser/fixtures/required-features.wat. Uses a GC
// structure, typed function reference, and return_call_ref in one tiny module.
const featureProbe = Uint8Array.from(atob("AGFzbQEAAAABDgNfAX8AYAF/AX9gAAF/AwMCAQIHCQEFcHJvYmUAAQkFAQMAAQAKGQIHACAAQQFqCw8AQSn7AAD7AgAA0gAVAQsALgRuYW1lAQwBAAlpbmNyZW1lbnQCCgEAAQAFdmFsdWUEDQIAA2JveAEFdW5hcnk="), (value) => value.charCodeAt(0));

export function supportsRequiredFeatures() {
  return typeof WebAssembly === "object" && WebAssembly.validate(featureProbe);
}

export class iLangPanic extends Error {
  constructor(message) {
    super(message);
    this.name = "iLangPanic";
  }
}

export class iLangAbiError extends Error {
  constructor(message, options) {
    super(message, options);
    this.name = "iLangAbiError";
  }
}

function leb(value) {
  const bytes = [];
  do {
    const byte = value & 0x7f;
    value >>>= 7;
    bytes.push(byte | (value ? 0x80 : 0));
  } while (value);
  return bytes;
}

// Importing an exported Wasm function performs an exact function-type check.
// This tiny module also gives a JavaScript callback a concrete Wasm signature,
// so linking it into the guest checks metadata against the guest's imports.
// It works with precompiled modules without parsing recursive WasmGC types.
function typedFunction(fn, params, results, location) {
  const key = `${params.join(",")}:${results.join(",")}`;
  let module = signatureModules.get(key);
  if (!module) {
    const section = (id, bytes) => [id, ...leb(bytes.length), ...bytes];
    module = new WebAssembly.Module(Uint8Array.from([
      0, 97, 115, 109, 1, 0, 0, 0,
      ...section(1, [1, 0x60, ...leb(params.length), ...params, ...leb(results.length), ...results]),
      ...section(2, [1, 1, 97, 1, 102, 0, 0]), // import a.f with type 0
      ...section(7, [1, 1, 102, 0, 0]), // export f, the imported function
    ]));
    signatureModules.set(key, module);
  }
  try {
    return new WebAssembly.Instance(module, { a: { f: fn } }).exports.f;
  } catch (cause) {
    throw new iLangAbiError(`Wasm signature does not match ABI descriptor for ${location}`, { cause });
  }
}

function validateFunction(module, exports, name, params, results) {
  expect(typeof exports[name] === "function", `Missing exported function ${name}`);
  let signatures = checkedSignatures.get(module);
  if (!signatures) checkedSignatures.set(module, signatures = new Set());
  const key = JSON.stringify([name, params, results]);
  if (signatures.has(key)) return;
  typedFunction(exports[name], params, results, name);
  signatures.add(key);
}

function descriptor(value, location) {
  if (!value || Array.isArray(value) || !Array.isArray(value.params) || !kinds.has(value.result)) {
    throw new iLangAbiError(`Invalid function descriptor at ${location}`);
  }
  for (const type of value.params) {
    if (!kinds.has(type)) throw new iLangAbiError(`Unknown ABI type ${String(type)} at ${location}`);
  }
  return Object.freeze({ params: Object.freeze([...value.params]), result: value.result });
}

/** Read and validate the compiler's ilang.abi custom section. */
export function readAbi(module, supplied) {
  if (supplied === undefined && embeddedAbis.has(module)) return embeddedAbis.get(module);
  const sections = WebAssembly.Module.customSections(module, "ilang.abi");
  if (sections.length > 1) throw new iLangAbiError("Duplicate ilang.abi sections");
  let embedded;
  if (sections.length) {
    try {
      embedded = JSON.parse(decoder.decode(sections[0]));
    } catch (cause) {
      throw new iLangAbiError("Malformed ilang.abi metadata", { cause });
    }
  }
  if (sections.length && supplied !== undefined) throw new iLangAbiError("An embedded ABI cannot be overridden");
  const value = sections.length ? embedded : supplied === undefined ? { version: 1 } : supplied;
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new iLangAbiError("ABI metadata must be an object");
  if (value.version !== 1) throw new iLangAbiError(`Unsupported iLang ABI version ${value.version}`);
  for (const field of ["exports", "imports"]) {
    if (value[field] !== undefined && (!value[field] || typeof value[field] !== "object" || Array.isArray(value[field]))) {
      throw new iLangAbiError(`ABI ${field} must be an object`);
    }
  }
  const exports = Object.create(null);
  const imports = Object.create(null);
  for (const [name, signature] of Object.entries(value.exports ?? {})) {
    exports[name] = descriptor(signature, `export ${name}`);
  }
  for (const [name, functions] of Object.entries(value.imports ?? {})) {
    if (name === namespace) throw new iLangAbiError(`${namespace} is reserved for the runtime`);
    if (!functions || typeof functions !== "object" || Array.isArray(functions)) {
      throw new iLangAbiError(`Invalid import module ${name}`);
    }
    imports[name] = Object.create(null);
    for (const [field, signature] of Object.entries(functions)) {
      imports[name][field] = descriptor(signature, `import ${name}.${field}`);
    }
    Object.freeze(imports[name]);
  }
  if (value.entry !== undefined && value.entry !== null && typeof value.entry !== "string") {
    throw new iLangAbiError("ABI entry must be an export name");
  }
  const tests = value.tests === undefined ? [] : value.tests;
  if (!Array.isArray(tests) || tests.some((name) => typeof name !== "string")) {
    throw new iLangAbiError("ABI tests must be an array of export names");
  }
  const abi = Object.freeze({ version: 1, exports: Object.freeze(exports), imports: Object.freeze(imports), entry: value.entry ?? undefined, tests: Object.freeze([...tests]) });
  if (supplied === undefined) embeddedAbis.set(module, abi);
  return abi;
}

function unsigned(value, label) {
  if (typeof value !== "number" || !Number.isInteger(value) || value < -0x80000000 || value > 0xffffffff) {
    throw new iLangAbiError(`${label} must be a Wasm i32`);
  }
  return value >>> 0;
}

function expect(condition, message) {
  if (!condition) throw new iLangAbiError(message);
}

/** Encode an integer without converting it through a JavaScript Number. */
export function integerToLimbs(value) {
  if (typeof value !== "bigint") throw new TypeError("Int arguments must be JavaScript bigint values");
  const sign = value < 0n ? -1 : value > 0n ? 1 : 0;
  let magnitude = value < 0n ? -value : value;
  const limbs = [];
  while (magnitude !== 0n) {
    limbs.push(Number(magnitude & 0xffffffffn));
    magnitude >>= 32n;
  }
  const bytes = new Uint8Array(limbs.length * 4);
  const view = new DataView(bytes.buffer);
  limbs.forEach((limb, index) => view.setUint32(index * 4, limb, true));
  return { sign, bytes };
}

/** Decode the canonical sign/magnitude little-endian integer representation. */
export function integerFromLimbs(sign, bytes) {
  expect(sign === -1 || sign === 0 || sign === 1, "Invalid integer sign");
  expect(bytes instanceof Uint8Array && bytes.byteLength % 4 === 0, "Integer limbs must contain complete 32-bit words");
  expect((sign === 0) === (bytes.byteLength === 0), "Integer zero must have sign zero and no limbs");
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
  if (bytes.byteLength) expect(view.getUint32(bytes.byteLength - 4, true) !== 0, "Integer magnitude has leading zero limbs");
  let value = 0n;
  for (let offset = bytes.byteLength - 4; offset >= 0; offset -= 4) {
    value = (value << 32n) | BigInt(view.getUint32(offset, true));
  }
  return sign === -1 ? -value : value;
}

function bitArray(value) {
  expect(value && value.bytes instanceof Uint8Array, "BitArray requires { bytes: Uint8Array, bitLength: number }");
  const { bytes, bitLength } = value;
  expect(Number.isSafeInteger(bitLength) && bitLength >= 0 && bitLength <= 0xffffffff, "Invalid BitArray bit length");
  expect(bytes.length === Math.ceil(bitLength / 8), "BitArray byte length does not match its bit length");
  const remainder = bitLength % 8;
  if (remainder) expect((bytes.at(-1) & ((1 << (8 - remainder)) - 1)) === 0, "Unused BitArray bits must be zero");
  return { bytes, bitLength };
}

function defaultRandom(bytes) {
  if (!globalThis.crypto?.getRandomValues) throw new iLangAbiError("The host has no cryptographic random source");
  for (let offset = 0; offset < bytes.length; offset += 65536) {
    globalThis.crypto.getRandomValues(bytes.subarray(offset, Math.min(offset + 65536, bytes.length)));
  }
}

class Bridge {
  instance;

  exports() {
    if (!this.instance) {
      throw new iLangAbiError("Host memory access before instantiation; use __ilang_init instead of a Wasm start section");
    }
    return this.instance.exports;
  }

  memory() {
    const memory = this.exports().memory;
    if (!(memory instanceof WebAssembly.Memory)) throw new iLangAbiError("Guest does not export memory");
    return memory;
  }

  range(pointer, length) {
    const start = unsigned(pointer, "Pointer");
    const size = unsigned(length, "Length");
    const buffer = this.memory().buffer;
    expect(start + size <= buffer.byteLength, "Guest buffer is outside linear memory");
    return new Uint8Array(buffer, start, size);
  }

  copy(pointer, length) {
    return this.range(pointer, length).slice();
  }

  allocate(bytes, owned) {
    if (!bytes.length) return 0;
    const allocator = this.exports().__ilang_alloc;
    expect(typeof allocator === "function", "Guest does not export __ilang_alloc");
    const pointer = unsigned(allocator(bytes.length), "Allocation pointer");
    expect(pointer !== 0, "Guest allocation failed");
    this.range(pointer, bytes.length).set(bytes);
    owned.push([pointer, bytes.length]);
    return pointer;
  }

  release(buffers) {
    if (!buffers.length) return;
    const free = this.exports().__ilang_free;
    expect(typeof free === "function", "Guest does not export __ilang_free");
    const released = new Set();
    let error;
    for (const [pointer, length] of buffers.toReversed()) {
      if ((pointer === 0 && length === 0) || released.has(pointer)) continue;
      released.add(pointer);
      try { free(pointer, length); } catch (cause) { error ??= cause; }
    }
    if (error) throw error;
  }

  call(operation) {
    const exports = this.exports();
    const mark = exports.__ilang_abi_mark;
    const reset = exports.__ilang_abi_reset;
    expect((mark === undefined && reset === undefined) || (typeof mark === "function" && typeof reset === "function"), "Guest ABI frame exports must be provided as a pair");
    const token = mark?.();
    const owned = [];
    let value;
    let failure;
    let failed = false;
    try { value = operation(owned); } catch (error) { failed = true; failure = error; }
    try { this.release(owned); } catch (error) { if (!failed) { failed = true; failure = error; } }
    if (reset) {
      try { reset(token); } catch (error) { if (!failed) { failed = true; failure = error; } }
    }
    if (failed) throw failure;
    return value;
  }

  lower(type, value, owned) {
    switch (type) {
      case "nil":
        expect(value === undefined || value === null, "Nil must be undefined or null");
        return [];
      case "bool":
        expect(typeof value === "boolean", "Bool arguments must be booleans");
        return [value ? 1 : 0];
      case "float":
        expect(typeof value === "number" && Number.isFinite(value), "Float arguments must be finite numbers");
        return [value];
      case "int": {
        const { sign, bytes } = integerToLimbs(value);
        return [sign, this.allocate(bytes, owned), bytes.length / 4];
      }
      case "string": {
        expect(typeof value === "string", "String arguments must be strings");
        // Lone surrogates are not Unicode scalar values. TextEncoder replaces
        // them silently; rejecting here keeps the API lossless.
        expect(value.isWellFormed(), "String contains an unpaired UTF-16 surrogate");
        const bytes = encoder.encode(value);
        return [this.allocate(bytes, owned), bytes.length];
      }
      case "bit_array": {
        const { bytes, bitLength } = bitArray(value);
        return [this.allocate(bytes, owned), bitLength];
      }
      case "externref":
        return [value];
      default:
        throw new iLangAbiError(`Unsupported ABI type ${type}`);
    }
  }

  lift(type, values, owned) {
    expect(values.length === arities[type], `Invalid result arity for ${type}`);
    switch (type) {
      case "nil": return undefined;
      case "bool":
        expect(values[0] === 0 || values[0] === 1, "Invalid Bool representation");
        return values[0] === 1;
      case "float":
        expect(typeof values[0] === "number" && Number.isFinite(values[0]), "Guest returned a non-finite Float");
        return values[0];
      case "externref": return values[0];
      case "string": {
        const [pointer, length] = values;
        const bytes = this.copy(pointer, length);
        if (owned) owned.push([unsigned(pointer, "Pointer"), bytes.length]);
        try { return decoder.decode(bytes); } catch (cause) {
          throw new iLangAbiError("Guest returned invalid UTF-8", { cause });
        }
      }
      case "int": {
        const [sign, pointer, count] = values;
        const byteLength = unsigned(count, "Limb count") * 4;
        expect(byteLength <= 0xffffffff, "Integer limb count overflows ABI memory");
        const bytes = this.copy(pointer, byteLength);
        if (owned) owned.push([unsigned(pointer, "Pointer"), bytes.length]);
        return integerFromLimbs(sign, bytes);
      }
      case "bit_array": {
        const [pointer, rawBitLength] = values;
        const bitLength = unsigned(rawBitLength, "Bit length");
        const bytes = this.copy(pointer, Math.ceil(bitLength / 8));
        if (owned) owned.push([unsigned(pointer, "Pointer"), bytes.length]);
        return bitArray({ bytes, bitLength });
      }
      default: throw new iLangAbiError(`Unsupported ABI type ${type}`);
    }
  }
}

function resultValues(type, value) {
  if (arities[type] === 0) {
    expect(value === undefined, "Nil functions must not return Wasm values");
    return [];
  }
  if (arities[type] === 1) return [value];
  expect(Array.isArray(value), `Expected multiple Wasm results for ${type}`);
  return value;
}

function importResult(values) {
  return values.length === 0 ? undefined : values.length === 1 ? values[0] : values;
}

async function compile(source) {
  if (source instanceof WebAssembly.Module) return source;
  if (typeof source === "string" || source instanceof URL || source instanceof Request) source = await fetch(source);
  if (source instanceof Response) {
    if (!source.ok) throw new iLangAbiError(`Unable to fetch Wasm: HTTP ${source.status}`);
    source = await source.arrayBuffer();
  }
  try { return await WebAssembly.compile(source); } catch (cause) {
    throw new iLangAbiError("Unable to compile iLang module; the engine must support WasmGC, typed function references and tail calls", { cause });
  }
}

/**
 * Load a module, bind its imports, then run explicit initialization exactly once.
 * Imports declared in metadata take ordinary JavaScript values; rawImports is
 * available for low-level Wasm imports whose ABI is managed by the caller.
 */
export async function instantiate(source, options = {}) {
  if (!supportsRequiredFeatures()) {
    throw new iLangAbiError("This engine does not support WasmGC, typed function references and tail calls");
  }
  const module = await compile(source);
  const abi = readAbi(module, options.abi);
  const bridge = new Bridge();
  const stdout = options.stdout ?? ((text) => console.log(text));
  const stderr = options.stderr ?? ((text) => console.error(text));
  const random = options.random ?? defaultRandom;
  const imports = Object.create(null);
  const objectIds = new WeakMap();
  const primitiveIds = new Map();
  let nextReferenceId = 1n;
  for (const [name, values] of Object.entries(options.rawImports ?? {})) {
    expect(name !== namespace, `${namespace} imports are managed by the loader`);
    imports[name] = { ...values };
  }
  imports[namespace] = {
    reference_id(value) {
      if (value === null) return 0n;
      const identities = (typeof value === "object" || typeof value === "function") ? objectIds : primitiveIds;
      let id = identities.get(value);
      if (id === undefined) {
        expect(nextReferenceId <= 0x7fffffffffffffffn, "Host reference identity space exhausted");
        id = nextReferenceId++;
        identities.set(value, id);
      }
      return id;
    },
    write(channel, pointer, length) {
      expect(channel === 1 || channel === 2, `Unknown output channel ${channel}`);
      const text = decoder.decode(bridge.copy(pointer, length));
      (channel === 1 ? stdout : stderr)(text);
    },
    random(pointer, length) {
      // Give the provider owned memory: a callback can re-enter the guest and
      // grow its memory without detaching the bytes currently being filled.
      bridge.range(pointer, length);
      const bytes = new Uint8Array(unsigned(length, "Random buffer length"));
      const result = random(bytes);
      expect(!result || typeof result.then !== "function", "The random provider must be synchronous");
      bridge.range(pointer, length).set(bytes);
      return 0;
    },
    panic(pointer, length) {
      throw new iLangPanic(decoder.decode(bridge.copy(pointer, length)));
    },
  };
  for (const [name, params, results] of [
    ["reference_id", [0x6f], [0x7e]],
    ["write", [0x7f, 0x7f, 0x7f], []],
    ["random", [0x7f, 0x7f], [0x7f]],
    ["panic", [0x7f, 0x7f], []],
  ]) imports[namespace][name] = typedFunction(imports[namespace][name], params, results, `${namespace}.${name}`);
  for (const [moduleName, functions] of Object.entries(abi.imports)) {
    imports[moduleName] ??= Object.create(null);
    for (const [name, signature] of Object.entries(functions)) {
      const fn = options.imports?.[moduleName]?.[name];
      expect(typeof fn === "function", `Missing host function ${moduleName}.${name}`);
      expect(!Object.hasOwn(imports[moduleName], name), `Host function ${moduleName}.${name} is registered twice`);
      const callback = (...rawArguments) => {
        const args = [];
        let offset = 0;
        for (const type of signature.params) {
          const arity = arities[type];
          args.push(bridge.lift(type, rawArguments.slice(offset, offset + arity)));
          offset += arity;
        }
        expect(offset === rawArguments.length, `Invalid argument arity for ${moduleName}.${name}`);
        const result = fn(...args);
        expect(!result || typeof result.then !== "function", "iLang ABI v1 host functions must be synchronous");
        const owned = [];
        try {
          const values = bridge.lower(signature.result, result, owned);
          // Successfully returned buffers transfer to the guest caller.
          return importResult(values);
        } catch (error) {
          try { bridge.release(owned); } catch { /* Preserve the primary ABI error. */ }
          throw error;
        }
      };
      imports[moduleName][name] = typedFunction(callback, signature.params.flatMap((type) => wasmTypes[type]), wasmTypes[signature.result], `${moduleName}.${name}`);
    }
  }
  const instance = await WebAssembly.instantiate(module, imports);
  bridge.instance = instance;
  for (const [name, signature] of Object.entries(abi.exports)) {
    validateFunction(module, instance.exports, name, signature.params.flatMap((type) => wasmTypes[type]), wasmTypes[signature.result]);
  }
  for (const name of [...abi.tests, ...(abi.entry ? [abi.entry] : [])]) validateFunction(module, instance.exports, name, [], []);
  for (const [name, params, results] of [
    ["__ilang_alloc", [0x7f], [0x7f]],
    ["__ilang_free", [0x7f, 0x7f], []],
    ["__ilang_abi_mark", [], [0x7f]],
    ["__ilang_abi_reset", [0x7f], []],
    ["__ilang_init", [], []],
  ]) if (instance.exports[name] !== undefined) validateFunction(module, instance.exports, name, params, results);
  if (instance.exports.__ilang_init !== undefined) {
    expect(typeof instance.exports.__ilang_init === "function", "__ilang_init must be a function");
    expect(instance.exports.__ilang_init.length === 0, "__ilang_init must have no Wasm parameters");
    resultValues("nil", instance.exports.__ilang_init());
  }
  return Object.freeze({
    module, instance, exports: instance.exports, abi,
    call(name, args = []) {
      let signature = abi.exports[name];
      if (!signature && (name === abi.entry || abi.tests.includes(name))) signature = { params: [], result: "nil" };
      expect(signature, `No ABI descriptor for exported function ${name}`);
      const fn = instance.exports[name];
      expect(typeof fn === "function", `Missing exported function ${name}`);
      expect(Array.isArray(args) && args.length === signature.params.length, `${name} expects ${signature.params.length} arguments`);
      const rawArity = signature.params.reduce((sum, type) => sum + arities[type], 0);
      expect(fn.length === rawArity, `Wasm parameter count does not match ABI descriptor for ${name}`);
      return bridge.call((owned) => {
        const rawArguments = signature.params.flatMap((type, index) => bridge.lower(type, args[index], owned));
        return bridge.lift(signature.result, resultValues(signature.result, fn(...rawArguments)), owned);
      });
    },
    run() {
      const name = abi.entry ?? "_start";
      const fn = instance.exports[name];
      expect(typeof fn === "function", `Missing entry export ${name}`);
      validateFunction(module, instance.exports, name, [], []);
      bridge.call(() => resultValues("nil", fn()));
    },
  });
}

/** Run each test in a fresh instance so a trap cannot poison later tests. */
export async function runTests(source, options = {}) {
  const module = await compile(source);
  const abi = readAbi(module, options.abi);
  const names = options.tests ?? abi.tests;
  const results = [];
  for (const name of names) {
    try {
      const guest = await instantiate(module, options);
      expect(typeof guest.exports[name] === "function", `Missing test export ${name}`);
      validateFunction(module, guest.exports, name, [], []);
      resultValues("nil", guest.exports[name]());
      results.push({ name, passed: true });
    } catch (error) {
      results.push({ name, passed: false, error });
    }
  }
  return results;
}
