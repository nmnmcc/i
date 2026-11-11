import assert from "node:assert/strict";
import test from "node:test";
import { AbiError, readCollections } from "../ilang.mjs";

const encode = new TextEncoder();
function leb(n) {
  const out = [];
  do { const byte = n & 127; n >>>= 7; out.push(byte | (n ? 128 : 0)); } while (n);
  return out;
}
function section(value) {
  const name = encode.encode("ilang.collections");
  const payload = encode.encode(JSON.stringify(value));
  const body = [...leb(name.length), ...name, ...payload];
  return [0, ...leb(body.length), ...body];
}
function moduleWith(...sections) {
  return new WebAssembly.Module(Uint8Array.from([0, 97, 115, 109, 1, 0, 0, 0, ...sections.flat()]));
}
test("reads and validates selected collection metadata", () => {
  const metadata = { manifest: { revision: "r", entries: [{ handle: "r:0" }] }, selected: "r:0", export: "i.tool.0", is_async: true };
  assert.deepEqual(readCollections(moduleWith(section(metadata))), metadata);
  assert.equal(readCollections(moduleWith()), null);
  assert.throws(() => readCollections(moduleWith(section({ ...metadata, selected: "r:1" }))), AbiError);
  assert.throws(() => readCollections(moduleWith(section(metadata), section(metadata))), AbiError);
});
