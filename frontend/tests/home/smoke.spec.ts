// spec: issue #474 — 首页拆分为纯静态个人记录落地页（备案整改）
import { test, expect } from '../helpers/graphql-aware';
import { login, SPACE_BASE } from '../helpers/auth';

test.describe('Landing - Smoke', () => {
  test('Landing page loads as static personal learning record', { tag: '@smoke' }, async ({ page }) => {
    // 跟踪 /graphql 与 /api 请求 — 落地页应为零 API 纯静态（忽略 Vite 的 /src/api/*.ts 静态资源）
    const apiRequests: string[] = [];
    page.on('request', (req) => {
      try {
        const { pathname } = new URL(req.url());
        if (pathname.startsWith('/api') || pathname.startsWith('/graphql')) {
          apiRequests.push(req.url());
        }
      } catch {
        // ignore non-parsable URLs (data:, blob:)
      }
    });

    await page.goto('/');

    // 个人记录口径：标题与 Hero 使用「个人技术学习记录」
    await expect(page.getByRole('heading', { name: '个人技术学习记录' })).toBeVisible();

    // 去企业化：不出现企业版/SaaS/商业服务字样
    await expect(page.getByText(/企业版|SaaS|商业服务/)).not.toBeVisible();

    // 三个静态板块可见：学习方向 / 实践项目 / 复盘
    await expect(page.getByRole('heading', { name: '学习方向' })).toBeVisible();
    await expect(page.getByRole('heading', { name: '实践项目' })).toBeVisible();
    await expect(page.getByRole('heading', { name: '复盘' })).toBeVisible();

    // footer 保留备案号链接与「个人技术项目」字样
    await expect(page.getByText(/个人技术项目/)).toBeVisible();
    await expect(page.getByRole('link', { name: /粤ICP备2025471124号/ })).toBeVisible();

    // 落地页不发起任何 GraphQL/API 请求（纯静态）
    expect(apiRequests).toHaveLength(0);
  });

  test('Landing CTA navigates to login', { tag: '@smoke' }, async ({ page }) => {
    await page.goto('/');

    // 落地页仅保留单个「进入平台」CTA，指向 /login
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
