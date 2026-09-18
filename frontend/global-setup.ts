// CI readiness gate: wait for frontend + backend before running tests.
// Cold dev images (cargo watch debug compile) need minutes on first boot;
// without this the suite fails fast with 100+ confusing errors.
import { request } from '@playwright/test';

async function waitFor(url: string, label: string, timeoutMs: number) {
  const deadline = Date.now() + timeoutMs;
  let last = 'no attempts';
  while (Date.now() < deadline) {
    const ctx = await request.newContext();
    try {
      const res = await ctx.get(url, { timeout: 10000 });
      if (res.status() < 500) {
        await ctx.dispose();
        return;
      }
      last = `status=${res.status()}`;
    } catch (e) {
      last = e instanceof Error ? e.message.split('\n')[0].slice(0, 120) : 'error';
    } finally {
      await ctx.dispose().catch(() => {});
    }
    await new Promise((r) => setTimeout(r, 3000));
  }
  throw new Error(`[global-setup] ${label} not ready: ${url} last=${last}`);
}

export default async function globalSetup() {
  const base = process.env.E2E_BASE_URL || (process.env.CI ? 'http://localhost:80' : 'http://localhost:3000');
  const backend = process.env.E2E_BACKEND_URL || 'http://localhost:8080';
  await waitFor(`${base}/health`, 'frontend', 6 * 60 * 1000);
  await waitFor(`${backend}/health`, 'backend', 12 * 60 * 1000);
}
