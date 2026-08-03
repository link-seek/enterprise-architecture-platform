// spec: 移动端适配 - 讨论issue "移动端适配"
import { test, expect, type Page } from '@playwright/test';
import { login, SPACE_BASE, TEST_EMAIL } from '../helpers/auth';

const MOBILE = { width: 375, height: 667 };
const noOverflow = (page: Page) =>
  page.evaluate(
    () => document.documentElement.scrollWidth - document.documentElement.clientWidth
  );

test.describe('Mobile Responsive Layout', () => {
  test.beforeEach(async ({ page }) => {
    await page.setViewportSize(MOBILE);
    await login(page);
  });

  test('移动端无横向溢出 - 价值流页', async ({ page }) => {
    // Already at value-streams after login.
    await expect(page.getByRole('heading', { name: '价值流' }).first()).toBeVisible({ timeout: 10000 });
    expect(await noOverflow(page)).toBeLessThanOrEqual(2);
  });

  test('移动端无横向溢出 - 业务能力页', async ({ page }) => {
    // Use in-app navigation to avoid full page reload (more stable, no re-auth needed).
    await page.getByRole('button', { name: /打开菜单|菜单|menu/i }).click();
    await page.getByRole('link', { name: '业务能力' }).first().click();
    await expect(page).toHaveURL(`${SPACE_BASE}/capabilities`, { timeout: 10000 });
    await expect(page.getByRole('heading', { name: '业务能力' }).first()).toBeVisible({ timeout: 10000 });
    expect(await noOverflow(page)).toBeLessThanOrEqual(2);
  });

  test('移动端无横向溢出 - 业务流程页', async ({ page }) => {
    await page.getByRole('button', { name: /打开菜单|菜单|menu/i }).click();
    await page.getByRole('link', { name: '业务流程' }).first().click();
    await expect(page).toHaveURL(`${SPACE_BASE}/processes`, { timeout: 10000 });
    await expect(page.getByRole('heading', { name: '业务流程' }).first()).toBeVisible({ timeout: 10000 });
    expect(await noOverflow(page)).toBeLessThanOrEqual(2);
  });

  test('移动端侧边栏抽屉开关', async ({ page }) => {
    await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`);
    await expect(page.getByRole('link', { name: '业务流程' })).not.toBeVisible();

    await page.getByRole('button', { name: /打开菜单|菜单|menu/i }).click();
    await expect(page.getByRole('link', { name: '业务流程' }).first()).toBeVisible({ timeout: 3000 });

    await page.keyboard.press('Escape');
    await expect(page.getByRole('link', { name: '业务流程' })).not.toBeVisible({ timeout: 3000 });
  });

  test('移动端抽屉导航后自动关闭', async ({ page }) => {
    await expect(page.getByRole('button', { name: /打开菜单|菜单|menu/i })).toBeVisible({ timeout: 10000 });
    await page.getByRole('button', { name: /打开菜单|菜单|menu/i }).click();
    await page.getByRole('link', { name: '业务流程' }).first().click();
    await expect(page).toHaveURL(`${SPACE_BASE}/processes`);
    await expect(page.getByRole('link', { name: '价值流' })).not.toBeVisible({ timeout: 3000 });
  });

  test('移动端用户信息与退出登录可达', async ({ page }) => {
    await page.getByRole('button', { name: /打开菜单|菜单|menu/i }).click();
    // Scope to the visible dialog to avoid matching the hidden desktop sidebar copy.
    const dialog = page.getByRole('dialog');
    await expect(dialog.getByText(TEST_EMAIL)).toBeVisible({ timeout: 3000 });
    await expect(dialog.getByRole('button', { name: '退出登录' })).toBeVisible();
  });
});

test.describe('Mobile Login Page', () => {
  test('登录页在 375px 屏幕不溢出且表单可用', async ({ page }) => {
    await page.setViewportSize(MOBILE);
    await page.goto('/login');
    await expect(page.getByText('企业架构平台')).toBeVisible();
    expect(await noOverflow(page)).toBeLessThanOrEqual(2);
    await expect(page.getByRole('textbox', { name: '邮箱' })).toBeVisible();
    await expect(page.getByRole('textbox', { name: '密码' })).toBeVisible();
  });
});

test.describe('Mobile Home & Spaces', () => {
  test('首页在 375px 屏幕无横向溢出', async ({ page }) => {
    await page.setViewportSize(MOBILE);
    await page.goto('/');
    await expect(page.getByRole('heading', { name: '企业架构平台' })).toBeVisible();
    expect(await noOverflow(page)).toBeLessThanOrEqual(2);
  });

  test('空间列表在 375px 屏幕无横向溢出', async ({ page }) => {
    await page.setViewportSize(MOBILE);
    await page.goto('/spaces');
    await expect(page.getByRole('heading', { name: '所有空间' })).toBeVisible({ timeout: 10000 });
    expect(await noOverflow(page)).toBeLessThanOrEqual(2);
  });
});