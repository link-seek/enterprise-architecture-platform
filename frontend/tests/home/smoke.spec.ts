// spec: issue #297 — 首页品牌回归与真实架构概览数据展示
import { test, expect } from '../helpers/graphql-aware';

test.describe('Home - Smoke', () => {
  test('Home page loads with enterprise brand and real data', { tag: '@smoke' }, async ({ page }) => {
    await page.goto('/');

    // 品牌名「企业架构平台」在 header 与 Hero H1 中可见
    await expect(page.getByRole('heading', { name: '企业架构平台' })).toBeVisible();

    // 浏览器标题回归企业架构口径
    await expect(page).toHaveTitle('企业架构平台');

    // Hero 副标题不再出现技术栈字样
    await expect(page.getByText(/Rust|React|全栈/)).not.toBeVisible();

    // CTA「浏览架构空间」跳转到公开的 /spaces
    const cta = page.getByRole('link', { name: '浏览架构空间' });
    await expect(cta).toBeVisible();

    // 架构概览数据区加载完成，不出现「加载失败」
    await expect(page.getByText(/加载失败/)).not.toBeVisible({ timeout: 10000 });

    // 平台概览数字区可见（空间 / 价值流 / 业务能力 / 业务流程）
    await expect(page.getByText('架构概览')).toBeVisible();

    // footer 保留备案号链接与「个人技术项目」字样
    await expect(page.getByText(/个人技术项目/)).toBeVisible();
    await expect(page.getByRole('link', { name: /粤ICP备2025471124号/ })).toBeVisible();
  });

  test('Home CTA navigates to public spaces list', { tag: '@smoke' }, async ({ page }) => {
    await page.goto('/');

    const cta = page.getByRole('link', { name: '浏览架构空间' });
    await expect(cta).toBeVisible();
    await cta.click();
    await expect(page).toHaveURL('/spaces');
  });
});