import { test, expect } from '@playwright/test';
import { login, SPACE_BASE } from '../helpers/auth';

test.describe('v2.0 MVP - New Application Architecture Pages', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('应用组件 page loads via sidebar', { tag: '@smoke' }, async ({ page }) => {
    await page.getByRole('link', { name: '应用组件', exact: true }).click();
    await expect(page).toHaveURL(`${SPACE_BASE}/applications`);
    await expect(page.getByRole('heading', { name: '应用组件' })).toBeVisible();
  });

  test('应用流程 page loads via sidebar', { tag: '@smoke' }, async ({ page }) => {
    await page.getByRole('link', { name: '应用流程', exact: true }).click();
    await expect(page).toHaveURL(`${SPACE_BASE}/application-processes`);
    await expect(page.getByRole('heading', { name: '应用流程' })).toBeVisible();
  });

  test('映射关系 page loads via sidebar', { tag: '@smoke' }, async ({ page }) => {
    await page.getByRole('link', { name: '映射关系' }).click();
    await expect(page).toHaveURL(`${SPACE_BASE}/realizations`);
    await expect(page.getByRole('heading', { name: '映射关系' })).toBeVisible();
    await expect(page.getByText('业务能力 → 流程（v2.1）')).toBeVisible();
  });

  test('应用组件 create dialog opens and submits', { tag: '@regression' }, async ({ page }) => {
    await page.getByRole('link', { name: '应用组件', exact: true }).click();
    await page.getByRole('button', { name: '新建组件' }).click();
    const dialog = page.getByRole('dialog');
    await expect(dialog.getByRole('heading', { name: '新建组件' })).toBeVisible();
    const name = `e2e-comp-${Date.now()}`;
    await dialog.getByRole('textbox').first().fill(name);
    await dialog.getByPlaceholder('org/repo').fill('e2e/repo');
    await dialog.getByPlaceholder('src/service').fill('src/e2e');
    await dialog.getByRole('button', { name: '创建' }).click();
    await expect(page.getByText(name)).toBeVisible({ timeout: 10000 });
  });

  test('应用流程 create dialog opens and submits', { tag: '@regression' }, async ({ page }) => {
    await page.getByRole('link', { name: '应用流程', exact: true }).click();
    await page.getByRole('button', { name: '新建流程' }).click();
    const dialog = page.getByRole('dialog');
    await expect(dialog.getByRole('heading', { name: '新建流程' })).toBeVisible();
    const name = `e2e-proc-${Date.now()}`;
    await dialog.getByRole('textbox').first().fill(name);
    await dialog.getByRole('button', { name: '创建' }).click();
    await expect(page.getByText(name)).toBeVisible({ timeout: 10000 });
  });
});