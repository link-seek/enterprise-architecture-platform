// spec: frontend/src/views/landing.tsx
import { test, expect } from '../helpers/graphql-aware';
import { login, SPACE_BASE } from '../helpers/auth';

test.describe('Landing View - Smoke', () => {
  test('Landing page loads as static personal learning record', { tag: '@smoke' }, async ({ page }) => {
    const apiRequests: string[] = [];
    page.on('request', (req) => {
      const url = req.url();
      if (url.includes('/graphql') || url.includes('/api')) {
        apiRequests.push(url);
      }
    });

    await page.goto('/');

    await expect(page.getByRole('heading', { name: '个人技术学习记录' })).toBeVisible();
    await expect(page.getByText(/企业版|SaaS|商业服务/)).not.toBeVisible();
    await expect(page.getByRole('heading', { name: '学习方向' })).toBeVisible();
    await expect(page.getByRole('heading', { name: '实践项目' })).toBeVisible();
    await expect(page.getByRole('heading', { name: '复盘' })).toBeVisible();
    await expect(page.getByText(/个人技术项目/)).toBeVisible();
    await expect(page.getByRole('link', { name: /粤ICP备2025471124号/ })).toBeVisible();
    expect(apiRequests).toHaveLength(0);
  });

  test('Landing CTA navigates to login', { tag: '@smoke' }, async ({ page }) => {
    await page.goto('/');
    const cta = page.getByRole('link', { name: '进入平台' });
    await expect(cta).toBeVisible();
    await cta.click();
    await expect(page).toHaveURL('/login');
  });

  test('Authenticated user is redirected from landing to overview', { tag: '@smoke' }, async ({ page }) => {
    await login(page);
    await page.goto('/');
    await expect(page).toHaveURL(`${SPACE_BASE}/overview`);
  });
});
