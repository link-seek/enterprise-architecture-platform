import { defineConfig } from "@playwright/test";

/**
 * Playwright config for EAP Frontend E2E tests.
 * - E2E_BASE_URL set: test against external server (e.g. production smoke test)
 * - CI: tests against nginx in docker-compose.ci.yml (port 80)
 * - Local: tests against vite dev server (port 3000)
 */
const isCI = !!process.env.CI;
const hasExternalServer = !!process.env.E2E_BASE_URL;
const baseURL = process.env.E2E_BASE_URL || (isCI ? "http://localhost:80" : "http://localhost:3000");

export default defineConfig({
  testDir: "./tests",
  fullyParallel: false,
  workers: 1,
  reporter: "line",
  timeout: 90_000,

  use: {
    baseURL,
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    viewport: { width: 1280, height: 720 },
  },

  // No webServer when testing against an external server or CI docker.
  webServer: hasExternalServer || isCI
    ? undefined
    : {
        command: "npm run dev",
        url: "http://localhost:3000",
        timeout: 90_000,
        reuseExistingServer: true,
      },
});
