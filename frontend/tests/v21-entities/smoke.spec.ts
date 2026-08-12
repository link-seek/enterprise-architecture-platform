import { test, expect } from '../helpers/graphql-aware';
import { login, SPACE_BASE } from '../helpers/auth';

test.describe('v2.1 New Entity Pages - Smoke Tests', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('Organizational Units page loads', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/organizational-units`);
    await expect(page.getByRole('heading', { name: '组织单元', exact: true })).toBeVisible();
    await expect(page.getByText('组织单元列表')).toBeVisible();
  });

  test('Business Roles page loads', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/business-roles`);
    await expect(page.getByRole('heading', { name: '业务角色' })).toBeVisible();
    await expect(page.getByText('角色列表')).toBeVisible();
  });

  test('Functional Modules page loads', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/functional-modules`);
    await expect(page.getByRole('heading', { name: '功能模块' })).toBeVisible();
    await expect(page.getByText('模块列表')).toBeVisible();
  });

  test('Application Interfaces page loads', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/application-interfaces`);
    await expect(page.getByRole('heading', { name: '应用接口' })).toBeVisible();
    await expect(page.getByText('接口列表')).toBeVisible();
  });

  test('Sidebar contains new entity links', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/value-streams`);
    await expect(page.getByRole('link', { name: '组织单元' })).toBeVisible();
    await expect(page.getByRole('link', { name: '业务角色' })).toBeVisible();
    await expect(page.getByRole('link', { name: '功能模块' })).toBeVisible();
    await expect(page.getByRole('link', { name: '应用接口' })).toBeVisible();
  });
});