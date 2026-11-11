import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { AbiError, instantiate } from "../ilang.mjs";

const [path, handle] = process.argv.slice(2);
const guest = await instantiate(await readFile(path));
assert.equal(guest.collections.selected, handle);
assert.throws(() => guest.runCollection(`${handle}-other`), AbiError);
await guest.runCollection(handle);
