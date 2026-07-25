// spec: specs/eap-test-plan.md
import { test, expect } from '@playwright/test';
import { login, ensureLoggedOut, SPACE_BASE } from '../helpers/auth';

test.describe('Authentication - Protected Routes', () => {
  test.beforeEach(async ({ page }) => {
    await ensureLoggedOut(page);
  });

  test('Unauthenticated user redirected from protected routes', async ({ page }) => {
    const protectedRoutes = [
      `${SPACE_BASE}/value-streams`,
      `${SPACE_BASE}/capabilities`,
      `${SPACE_BASE}/processes`,
    ];

    for (const route of protectedRoutes) {
      await page.goto(route);
      await expect(page).toHaveURL('/login');
      await expect(page.getByText('企业架构平台')).toBeVisible();
      await page.evaluate(() => localStorage.clear());
    }
  });

  test('Authenticated user can access protected routes', async ({ page }) => {
    await login(page);

    const routes = [
      { url: `${SPACE_BASE}/value-streams`, text: '价值流' },
      { url: `${SPACE_BASE}/capabilities`, text: '业务能力' },
      { url: `${SPACE_BASE}/processes`, text: '业务流程' },
    ];

    for (const r of routes) {
      await page.goto(r.url);
      await expect(page).toHaveURL(r.url);
      await expect(page.getByText(r.text)).toBeVisible({ timeout: 5000 });
    }
  });

  test('Session persistence across navigation', async ({ page }) => {
    await login(page);

    await page.goto(`${SPACE_BASE}/capabilities`);
    await expect(page).toHaveURL(`${SPACE_BASE}/capabilities`);

    await page.goto(`${SPACE_BASE}/processes`);
    await expect(page).toHaveURL(`${SPACE_BASE}/processes`);

    await page.goto(`${SPACE_BASE}/value-streams`);
    await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`);
  });

  test('Manual token removal triggers logout', async ({ page }) => {
    await login(page);

    await page.evaluate(() => localStorage.removeItem('access_token'));
    await page.reload();

    await expect(page).toHaveURL('/login', { timeout: 5000 });
  });
});
