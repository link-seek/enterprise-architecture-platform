import { defineConfig } from "@playwright/test";

/**
 * Playwright config for EAP Frontend E2E tests.
 * - E2E_BASE_URL set: test against an external server (production smoke,
 *   CI docker with nginx on port 80, etc.). No webServer is started.
 * - Otherwise: start the vite dev server on port 3000 and test against it.
 *   The dev server proxies /api and /graphql to the backend (port 8080).
 *   This covers local dev, CI verification, and any environment without a
 *   pre-existing frontend server.
 */
const hasExternalServer = !!process.env.E2E_BASE_URL;
const baseURL = process.env.E2E_BASE_URL || "http://localhost:3000";

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

  webServer: hasExternalServer
    ? undefined
    : {
        command: "npm run dev",
        url: "http://localhost:3000",
        timeout: 90_000,
        reuseExistingServer: true,
      },
});
