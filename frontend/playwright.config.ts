import { defineConfig } from "@playwright/test";

/**
 * Playwright config for EAP Frontend E2E tests.
 * - CI: tests against nginx in docker-compose.ci.yml (port 80)
 * - Local: tests against vite dev server (port 3000)
 */
const isCI = !!process.env.CI;
const baseURL = isCI ? "http://localhost:80" : "http://localhost:3000";

export default defineConfig({
  testDir: "./tests",
  fullyParallel: false,
  workers: 1,
  reporter: "line",
  timeout: 30_000,

  use: {
    baseURL,
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    viewport: { width: 1280, height: 720 },
  },

  // In CI, the frontend is already running in docker on port 80.
  // Locally, start vite dev server on port 3000.
  webServer: isCI
    ? undefined
    : {
        command: "npm run dev",
        url: "http://localhost:3000",
        timeout: 30_000,
        reuseExistingServer: true,
      },
});
