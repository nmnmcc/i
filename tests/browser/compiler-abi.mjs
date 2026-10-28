import { instantiate, WleamPanic } from "../../bindings/javascript/wleam.mjs";

// Shared by Node and browser verification of the real Gleam host_abi project.
export async function verifyCompilerAbi(source, options = {}) {
  const check = (condition, message) => { if (!condition) throw new Error(message); };
  const expectedHostFailure = new Error("intentional host callback failure");
  let failString = false;
  const names = ["echo_int", "echo_string", "echo_bits", "echo_ref", "echo_bool", "echo_float", "echo_nil"];
  const callbacks = Object.fromEntries(names.map((name) => [name, (value) => value]));
  callbacks.echo_string = (value) => {
    if (failString) throw expectedHostFailure;
    return value;
  };
  const guest = await instantiate(source, { ...options, imports: { custom: callbacks } });
  const call = (name, ...args) => guest.call(`host_abi.${name}`, args);
  for (const value of [0n, 1n, -1n, (1n << 30n) - 1n, 1n << 30n, -(1n << 64n), (1n << 4096n) + 0x123456789abcdefn]) {
    check(call("echo_int", value) === value, "Compiler Int import/export lost precision");
  }
  for (const value of ["", "e\u0301 👩🏽‍🚀 中文\0", "界".repeat(65536)]) {
    check(call("echo_string", value) === value, "Compiler String import/export changed UTF-8");
  }
  for (const value of [{ bytes: new Uint8Array(), bitLength: 0 }, { bytes: Uint8Array.of(255, 128), bitLength: 9 }]) {
    const result = call("echo_bits", value);
    check(result.bitLength === value.bitLength && result.bytes.length === value.bytes.length && result.bytes.every((byte, index) => byte === value.bytes[index]), "Compiler BitArray import/export changed bits");
  }
  const sliced = call("unaligned_bits");
  check(sliced.bitLength === 9 && sliced.bytes.length === 2 && sliced.bytes[0] === 255 && sliced.bytes[1] === 128, "Compiler did not normalize a nonzero bit offset");
  for (const value of [true, false]) check(call("echo_bool", value) === value, "Compiler Bool import/export changed value");
  for (const value of [-0, 1.25, Number.MIN_VALUE]) check(Object.is(call("echo_float", value), value), "Compiler Float import/export changed value");
  check(call("echo_nil", undefined) === undefined, "Compiler Nil import/export changed value");
  const reference = { opaque: true };
  const other = { opaque: true };
  for (const value of [reference, null, "primitive reference"]) check(call("echo_ref", value) === value, "Compiler opaque reference identity changed");
  check(call("reference_same", reference, reference), "Equal host references compared unequal");
  check(!call("reference_same", reference, other), "Distinct host references compared equal");
  check(call("reference_same", null, null), "Null host references compared unequal");
  check(!call("reference_same", reference, null), "Null and object host references compared equal");
  failString = true;
  for (let iteration = 0; iteration < 100; iteration++) {
    let caught;
    try { call("echo_string", "allocated guest import argument"); } catch (error) { caught = error; }
    check(caught === expectedHostFailure, "Compiler import changed the original host exception");
  }
  failString = false;
  check(call("echo_string", "recovered after host failures") === "recovered after host failures", "Guest did not recover after a host exception");
  let panic;
  try { call("panic_now"); } catch (error) { panic = error; }
  check(panic instanceof WleamPanic && panic.message.includes("host_abi intentional guest panic"), "Guest panic lost its message");
  check(call("echo_int", 42n) === 42n, "Guest did not recover after its panic");
  return { hostFailures: 100, guestPanic: panic.message };
}
