import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { expect, test } from "@playwright/test";

// Native conformance produces the manifest and these exact Wasm binaries.
// Missing inputs fail; host-only iteration is a separate named npm script.
const manifestPath = resolve(process.env.WLEAM_BROWSER_MANIFEST ?? "target/conformance/browser.json");
let manifest;
try {
  manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
} catch (cause) {
  throw new Error(`Compiler conformance manifest unavailable: ${manifestPath}. Run scripts/conformance.py first.`, { cause });
}
if (!Array.isArray(manifest) || !manifest.length) throw new Error("Compiler conformance manifest contains no fixtures");

for (const [index, fixture] of manifest.entries()) {
  if (typeof fixture.name !== "string" || typeof fixture.wasm !== "string") throw new Error(`Invalid compiler fixture ${index}`);
  test(`compiled: ${fixture.name}`, async ({ page }) => {
    // The full corpus runs thousands of exported tests, including deep recursion.
    test.setTimeout(10 * 60 * 1000);
    await page.goto("/tests/browser/harness.html");
    const result = await page.evaluate(async ({ index, names, hostAbi }) => {
      const { instantiate, runTests } = await import("/bindings/javascript/wleam.mjs");
      const stdout = [];
      const stderr = [];
      const options = { stdout: (text) => stdout.push(text), stderr: (text) => stderr.push(text) };
      const source = `/__compiled/${index}.wasm`;
      let tests;
      let hostResult;
      if (hostAbi) {
        const { verifyCompilerAbi } = await import("/tests/browser/compiler-abi.mjs");
        hostResult = await verifyCompilerAbi(source, options);
      } else if (names) {
        tests = (await runTests(source, { ...options, tests: names })).map(({ name, passed, error }) => ({ name, passed, error: error ? String(error.message ?? error) : undefined }));
      } else {
        const guest = await instantiate(source, options);
        guest.run();
      }
      return { stdout: stdout.join(""), stderr: stderr.join(""), tests, hostResult };
    }, { index, names: fixture.tests, hostAbi: fixture.hostAbi });
    expect(result.stdout).toBe(fixture.stdout ?? "");
    expect(result.stderr).toBe(fixture.stderr ?? "");
    if (fixture.hostAbi) {
      expect(result.hostResult.hostFailures).toBe(100);
      expect(result.hostResult.guestPanic).toContain("host_abi intentional guest panic");
    } else if (fixture.tests) {
      expect(result.tests).toHaveLength(fixture.tests.length);
      expect(result.tests.map(({ name }) => name)).toEqual(fixture.tests);
      expect(result.tests.filter(({ passed }) => !passed)).toEqual([]);
    }
  });
}
