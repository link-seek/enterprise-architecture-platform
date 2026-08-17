import { test, expect } from '../helpers/graphql-aware';
import { login, SPACE_BASE } from '../helpers/auth';

test.describe('Organizational Units - CRUD Operations', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.goto(`${SPACE_BASE}/organizational-units`);
  });

  test('Create organizational unit', { tag: ['@smoke', '@regression'] }, async ({ page }) => {
    const createButton = page.getByRole('button', { name: /新建组织单元/ });
    await expect(createButton).toBeVisible();
    await createButton.click();

    await expect(page.getByRole('dialog')).toBeVisible();
    const name = `测试组织单元_${Date.now()}`;
    await page.getByRole('textbox', { name: /名称/ }).fill(name);
    await page.getByRole('button', { name: '创建' }).click();

    await expect(page.getByText(name)).toBeVisible();
  });

  test('Delete organizational unit', { tag: ['@smoke', '@regression'] }, async ({ page }) => {
    // First create one to delete
    await page.getByRole('button', { name: /新建组织单元/ }).click();
    const name = `删除测试_${Date.now()}`;
    await page.getByRole('textbox', { name: /名称/ }).fill(name);
    await page.getByRole('button', { name: '创建' }).click();
    await expect(page.getByText(name)).toBeVisible();

    // Delete it
    const row = page.getByRole('row').filter({ hasText: name });
    await row.getByRole('button', { name: '删除' }).click();
    await expect(page.getByRole('heading', { name: '确认删除' })).toBeVisible();
    await page.getByRole('button', { name: '删除' }).click();

    await expect(page.getByText(name, { exact: true })).not.toBeVisible();
  });
});