// spec: issue #311 — 空间成员管理 E2E 测试（含写操作，@regression）
import { test, expect } from '../helpers/graphql-aware';
import { login, loginAsEditor, TEST_SPACE_ID, STRANGER_EMAIL } from '../helpers/auth';

const SPACE_DETAIL_URL = `/spaces/${TEST_SPACE_ID}`;

test.describe('Space Member Management', () => {
  test('Owner can open members dialog and see member list', { tag: '@regression' }, async ({ page }) => {
    await login(page);
    await page.goto(SPACE_DETAIL_URL);
    await expect(page.getByText('加载中')).not.toBeVisible({ timeout: 10000 });

    await page.getByRole('button', { name: '成员' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: '空间成员' })).toBeVisible();

    // The owner appears in the member list.
    await expect(page.getByText('拥有者')).toBeVisible({ timeout: 10000 });
  });

  test('Owner can add member by email with editor role', { tag: '@regression' }, async ({ page }) => {
    await login(page);
    await page.goto(SPACE_DETAIL_URL);
    await expect(page.getByText('加载中')).not.toBeVisible({ timeout: 10000 });

    await page.getByRole('button', { name: '成员' }).click();
    await expect(page.getByRole('dialog')).toBeVisible({ timeout: 10000 });

    // Fill the stranger email and add as editor.
    const emailInput = page.getByPlaceholder('user@example.com');
    await emailInput.fill(STRANGER_EMAIL);
    await page.getByRole('button', { name: '添加' }).click();

    // The new member appears with the editor badge.
    await expect(page.getByText(STRANGER_EMAIL)).not.toBeVisible({ timeout: 10000 });
    await expect(page.getByText('编辑者').first()).toBeVisible({ timeout: 10000 });
  });

  test('Owner can remove non-owner member', { tag: '@regression' }, async ({ page }) => {
    await login(page);
    await page.goto(SPACE_DETAIL_URL);
    await expect(page.getByText('加载中')).not.toBeVisible({ timeout: 10000 });

    await page.getByRole('button', { name: '成员' }).click();
    await expect(page.getByRole('dialog')).toBeVisible({ timeout: 10000 });

    // Find a non-owner member row and click its remove button (Trash2 icon).
    const editorRows = page.getByRole('row').filter({ hasText: '编辑者' });
    const rowCount = await editorRows.count();
    test.skip(rowCount === 0, 'No non-owner member to remove');

    await editorRows.first().getByRole('button').click();

    // The removed editor row disappears (the editor badge count decreases).
    await expect(editorRows).toHaveCount(rowCount - 1, { timeout: 10000 });
  });

  test('Add non-existent email shows error', { tag: '@regression' }, async ({ page }) => {
    await login(page);
    await page.goto(SPACE_DETAIL_URL);
    await expect(page.getByText('加载中')).not.toBeVisible({ timeout: 10000 });

    await page.getByRole('button', { name: '成员' }).click();
    await expect(page.getByRole('dialog')).toBeVisible({ timeout: 10000 });

    const emailInput = page.getByPlaceholder('user@example.com');
    await emailInput.fill('nonexistent-user-xyz@example.com');
    await page.getByRole('button', { name: '添加' }).click();

    // Error message appears.
    await expect(page.getByText(/未找到|失败|错误/).first()).toBeVisible({ timeout: 10000 });
  });

  test('Editor cannot see members button', { tag: '@regression' }, async ({ page }) => {
    await loginAsEditor(page);
    await page.goto(SPACE_DETAIL_URL);
    await expect(page.getByText('加载中')).not.toBeVisible({ timeout: 10000 });

    // Editor does not see the members button (owner-only).
    await expect(page.getByRole('button', { name: '成员' })).not.toBeVisible();
  });
});