import { readFile } from "node:fs/promises";
import { verifyCompilerAbi } from "../../../tests/browser/compiler-abi.mjs";

const path = process.argv[2] ?? "tests/conformance/projects/host_abi/build/wasm/host_abi.wasm";
await verifyCompilerAbi(await readFile(path));
console.log("Compiler host ABI verified in Node.js");
