import { bgLifecycleAbi as abi } from "./fixture-abi.mjs";

/** The same gate fixture and trusted test plan run on Node and all browsers. */
export async function bgLifecycleGuest(sdk, source, capture = () => {}, wait = () => new Promise(() => {}), resourceHost) {
  const bytes = typeof source === "string" ? await (await fetch(source)).arrayBuffer() : source;
  const digest = "sha256-" + [...new Uint8Array(await crypto.subtle.digest("SHA-256", bytes))].map((b) => b.toString(16).padStart(2, "0")).join("");
  const declarations = [];
  for (const [namespace, fields] of Object.entries(abi.services)) {
    for (const [name, signature] of Object.entries(fields)) declarations.push({ package: "fixture", module: "fixture/lifecycle", module_digest: digest, declaration: name, namespace, name, kind: "service", signature });
  }
  const bindings = await sdk.authorizeBindings(bytes, { artifact_digest: digest, declarations }, () => true, { abi });
  return sdk.instantiate(bindings.module, { abi, bindings, resourceHost, contextImports: {
    "bg:control/v1": { capture, end: (context) => { context.teardown(); } },
    "bg:work/v1": { wait },
  } });
}
export const bgCounter = (guest, offset) => new DataView(guest.exports.memory.buffer).getUint32(offset, true);
