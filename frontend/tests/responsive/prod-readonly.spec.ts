import { test, expect } from '@playwright/test';
const MOBILE = { width: 375, height: 667 };
const noOverflow = (page: import('@playwright/test').Page) =>
  page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth
  );

test.describe('Prod Readonly - Mobile', () => {
  test('首页移动端无横向溢出', async ({ page }) => {
    await page.setViewportSize(MOBILE);
    await page.goto('/');
    await expect(page.getByRole('heading', { name: '企业架构平台' })).toBeVisible();
    expect(await noOverflow(page)).toBeLessThanOrEqual(2);
  });

  test('空间列表移动端无横向溢出', async ({ page }) => {
    await page.setViewportSize(MOBILE);
    await page.goto('/spaces');
    await expect(page.getByRole('heading', { name: '所有空间' })).toBeVisible({ timeout: 10000 });
    expect(await noOverflow(page)).toBeLessThanOrEqual(2);
  });

  test('viewport meta 标签存在', async ({ page }) => {
    await page.goto('/');
    const meta = await page.locator('meta[name="viewport"]').getAttribute('content');
    expect(meta).toContain('width=device-width');
  });
});