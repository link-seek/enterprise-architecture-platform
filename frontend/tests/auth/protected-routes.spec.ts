// spec: specs/eap-test-plan.md
import { test, expect } from '@playwright/test';
import { login, ensureLoggedOut, SPACE_BASE } from '../helpers/auth';

test.describe('Authentication - Protected Routes', () => {
  test.beforeEach(async ({ page }) => {
    await ensureLoggedOut(page);
  });

  test('Unauthenticated user redirected from protected routes', { tag: '@smoke' }, async ({ page }) => {
    // Architecture pages are public read; only admin-only routes redirect.
    const protectedRoutes = [`${SPACE_BASE}/users`];

    for (const route of protectedRoutes) {
      await page.goto(route);
      await expect(page).toHaveURL('/login');
      await expect(page.getByText('企业架构平台')).toBeVisible();
      await page.evaluate(() => localStorage.clear());
    }
  });

  test('Anonymous user can access architecture pages', { tag: '@smoke' }, async ({ page }) => {
    const routes = [
      { url: `${SPACE_BASE}/value-streams`, text: '价值流' },
      { url: `${SPACE_BASE}/capabilities`, text: '业务能力' },
      { url: `${SPACE_BASE}/processes`, text: '业务流程' },
    ];

    for (const r of routes) {
      await page.goto(r.url);
      await expect(page).toHaveURL(r.url);
      await expect(page.getByText(r.text).first()).toBeVisible({ timeout: 10000 });
      await expect(page.getByText(/加载失败/)).not.toBeVisible({ timeout: 5000 });
    }
  });

  test('Authenticated user can access protected routes', { tag: '@smoke' }, async ({ page }) => {
    await login(page);

    const routes = [
      { url: `${SPACE_BASE}/value-streams`, text: '价值流' },
      { url: `${SPACE_BASE}/capabilities`, text: '业务能力' },
      { url: `${SPACE_BASE}/processes`, text: '业务流程' },
    ];

    for (const r of routes) {
      await page.goto(r.url);
      await expect(page).toHaveURL(r.url);
      await expect(page.getByText(r.text).first()).toBeVisible({ timeout: 5000 });
    }
  });

  test('Session persistence across navigation', { tag: '@smoke' }, async ({ page }) => {
    await login(page);

    await page.goto(`${SPACE_BASE}/capabilities`);
    await expect(page).toHaveURL(`${SPACE_BASE}/capabilities`);

    await page.goto(`${SPACE_BASE}/processes`);
    await expect(page).toHaveURL(`${SPACE_BASE}/processes`);

    await page.goto(`${SPACE_BASE}/value-streams`);
    await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`);
  });

  test('Manual token removal reflects logged-out state', { tag: '@smoke' }, async ({ page }) => {
    await login(page);

    await page.evaluate(() => localStorage.removeItem('access_token'));
    await page.reload();

    // Architecture pages are public read, so the page stays accessible and the
    // UI reflects the logged-out state (login prompt replaces logout button).
    await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`, { timeout: 5000 });
    await expect(page.getByRole('button', { name: '退出登录' })).not.toBeVisible();
    await expect(page.getByRole('link', { name: '登录' })).toBeVisible({ timeout: 5000 });
  });
});
