import { test, expect } from '@playwright/test';

const TEST_EMAIL = process.env.E2E_TEST_EMAIL || 'e2e3@test.com';
const TEST_PASSWORD = process.env.E2E_TEST_PASSWORD || 'e2e123456';

test.describe('Network debug', () => {
  for (let i = 1; i <= 3; i++) {
    test(`login attempt ${i}`, async ({ page }) => {
      page.on('request', (req) => {
        if (req.url().includes('/api/')) {
          console.log(`[REQ ${i}] ${req.method()} ${req.url()}`);
        }
      });
      page.on('response', async (res) => {
        if (res.url().includes('/api/')) {
          const status = res.status();
          let body = '';
          try { body = await res.text(); } catch {}
          console.log(`[RES ${i}] ${status} ${res.url()} body=${body.slice(0, 200)}`);
        }
      });

      await page.goto('/login');
      await page.fill('input[type="email"]', TEST_EMAIL);
      await page.fill('input[type="password"]', TEST_PASSWORD);
      await page.press('input[type="password"]', 'Enter');
      await page.waitForTimeout(3000);
      const url = page.url();
      console.log(`[URL ${i}] ${url}`);
      const errorEl = page.getByText('Login failed');
      const hasError = await errorEl.isVisible().catch(() => false);
      console.log(`[ERROR ${i}] hasError=${hasError}`);
    });
  }
});