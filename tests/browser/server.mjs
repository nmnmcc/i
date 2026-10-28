import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { dirname, extname, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const manifestPath = resolve(process.env.WLEAM_BROWSER_MANIFEST ?? `${root}/target/conformance/browser.json`);
const mime = { ".html": "text/html", ".mjs": "text/javascript", ".js": "text/javascript", ".json": "application/json", ".wasm": "application/wasm" };
const port = Number(process.env.WLEAM_BROWSER_PORT ?? 4179);

async function manifest() {
  const entries = JSON.parse(await readFile(manifestPath, "utf8"));
  if (!Array.isArray(entries) || !entries.length) throw new Error("Compiler conformance manifest is empty");
  for (const entry of entries) {
    if (typeof entry.name !== "string" || typeof entry.wasm !== "string") throw new Error("Invalid compiler conformance manifest entry");
  }
  return entries;
}

const server = createServer(async (request, response) => {
  try {
    const pathname = decodeURIComponent(new URL(request.url, `http://127.0.0.1:${port}`).pathname);
    if (pathname === "/__manifest") {
      response.setHeader("Content-Type", "application/json");
      response.end(JSON.stringify(await manifest()));
      return;
    }
    let file;
    if (pathname.startsWith("/__compiled/")) {
      const match = /^\/__compiled\/(\d+)\.wasm$/.exec(pathname);
      if (!match) throw new Error("Invalid compiler fixture route");
      const entry = (await manifest())[Number(match[1])];
      if (!entry) throw new Error("Unknown compiler fixture");
      file = resolve(root, entry.wasm);
    } else {
      file = resolve(root, `.${pathname}`);
      if (!file.startsWith(root + sep)) throw new Error("Path outside workspace");
    }
    const data = await readFile(file);
    response.setHeader("Content-Type", mime[extname(file)] ?? "application/octet-stream");
    response.setHeader("Cache-Control", "no-store");
    response.end(data);
  } catch (error) {
    response.writeHead(404, { "Content-Type": "text/plain" });
    response.end(error.message);
  }
});

server.listen(port, "127.0.0.1");
for (const signal of ["SIGINT", "SIGTERM"]) process.on(signal, () => server.close());
