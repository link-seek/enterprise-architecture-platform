// spec: views/landing — 个人技术学习记录落地页（对应 frontend/src/views/landing.tsx）
import { test, expect } from '../helpers/graphql-aware';
import { login, SPACE_BASE } from '../helpers/auth';

test.describe('Landing View', () => {
  test('Landing page renders personal record headings', { tag: '@smoke' }, async ({ page }) => {
    const apiRequests: string[] = [];
    page.on('request', (req) => {
      const url = req.url();
      // Match real API calls by URL path: in vite dev, JS module URLs like
      // `/src/api/auth.ts` contain the substring `/api` but are static
      // assets, not API calls. Check the pathname instead.
      let path = url;
      try {
        path = new URL(url).pathname;
      } catch {
        // Non-absolute URL (e.g. data:) — not an API call.
      }
      if (path.includes('/graphql') || path.startsWith('/api/') || path === '/api') apiRequests.push(url);
    });
    await page.goto('/');
    await expect(page.getByRole('heading', { name: '个人技术学习记录' })).toBeVisible();
    await expect(page.getByRole('heading', { name: '学习方向' })).toBeVisible();
    await expect(page.getByRole('heading', { name: '实践项目' })).toBeVisible();
    await expect(page.getByRole('heading', { name: '复盘' })).toBeVisible();
    await expect(page.getByRole('link', { name: '进入平台' })).toBeVisible();
    expect(apiRequests).toHaveLength(0);
  });

  test('Landing CTA navigates to login', { tag: '@smoke' }, async ({ page }) => {
    await page.goto('/');
    await page.getByRole('link', { name: '进入平台' }).click();
    await expect(page).toHaveURL('/login');
  });

  test('Authenticated redirect from landing to overview', { tag: '@smoke' }, async ({ page }) => {
    await login(page);
    await page.goto('/');
    await expect(page).toHaveURL(`${SPACE_BASE}/overview`);
  });
});
