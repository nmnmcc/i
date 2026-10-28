import { defineConfig } from "@playwright/test";

const port = Number(process.env.WLEAM_BROWSER_PORT ?? 4179);

export default defineConfig({
  testDir: "./tests/browser",
  testMatch: /.*\.spec\.mjs$/,
  fullyParallel: true,
  workers: 3,
  forbidOnly: Boolean(process.env.CI),
  retries: 0,
  reporter: [["list"], ["json", { outputFile: "target/browser/results.json" }]],
  outputDir: "target/browser/artifacts",
  use: { baseURL: `http://127.0.0.1:${port}`, headless: true },
  projects: [
    { name: "chromium", use: { browserName: "chromium" } },
    { name: "firefox", use: { browserName: "firefox" } },
    { name: "webkit", use: { browserName: "webkit" } },
  ],
  webServer: {
    command: "node tests/browser/server.mjs",
    url: `http://127.0.0.1:${port}/tests/browser/harness.html`,
    timeout: 15_000,
    reuseExistingServer: false,
  },
});
