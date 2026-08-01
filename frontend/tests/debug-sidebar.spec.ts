import { test, expect } from '@playwright/test';

const TEST_EMAIL = process.env.E2E_TEST_EMAIL || 'e2e3@test.com';
const TEST_PASSWORD = process.env.E2E_TEST_PASSWORD || 'e2e123456';
const TEST_SPACE_ID = process.env.E2E_TEST_SPACE_ID || '00000000-0000-0000-0000-000000000010';
const SPACE_BASE = `/spaces/${TEST_SPACE_ID}/architectures`;

test.describe('Sidebar debug', () => {
  test.beforeEach(async ({ page }) => {
    const idx = test.info().line;
    page.on('response', async (res) => {
      if (res.url().includes('/api/') || res.url().includes('/graphql')) {
        const status = res.status();
        if (status >= 400) {
          let body = '';
          try { body = await res.text(); } catch {}
          console.log(`[ERR L${idx}] ${status} ${res.url()} body=${body.slice(0, 300)}`);
        }
      }
    });

    console.log(`[LOGIN L${idx}] starting login`);
    await page.goto('/login');
    await page.fill('input[type="email"]', TEST_EMAIL);
    await page.fill('input[type="password"]', TEST_PASSWORD);
    await page.press('input[type="password"]', 'Enter');
    try {
      await expect(page.getByRole('link', { name: '价值流' })).toBeVisible({ timeout: 10000 });
      console.log(`[LOGIN L${idx}] success`);
    } catch (e) {
      console.log(`[LOGIN L${idx}] FAILED: ${(e as Error).message.slice(0, 200)}`);
      throw e;
    }
  });

  test('test A', { tag: '@smoke' }, async ({ page }) => {
    await expect(page.getByRole('link', { name: '价值流' })).toBeVisible();
    await expect(page.getByRole('link', { name: '业务能力' })).toBeVisible();
    await expect(page.getByRole('link', { name: '业务流程' })).toBeVisible();
    await page.getByRole('link', { name: '业务能力' }).click();
    await expect(page).toHaveURL(`${SPACE_BASE}/capabilities`);
    await page.getByRole('link', { name: '业务流程' }).click();
    await expect(page).toHaveURL(`${SPACE_BASE}/processes`);
    await page.getByRole('link', { name: '价值流' }).click();
    await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`);
  });

  test('test B', { tag: '@smoke' }, async ({ page }) => {
    await expect(page.getByRole('heading', { name: '价值流' }).first()).toBeVisible();
  });

  test('test C', { tag: '@smoke' }, async ({ page }) => {
    await expect(page.getByText(TEST_EMAIL)).toBeVisible();
  });
});