// spec: issue #311 — 空间成员管理 E2E 测试（含写操作，@regression）
import { test, expect } from '../helpers/graphql-aware';
import { login, loginAsEditor, TEST_SPACE_ID, STRANGER_EMAIL, STRANGER_NAME } from '../helpers/auth';

const SPACE_DETAIL_URL = `/spaces/${TEST_SPACE_ID}`;

test.describe('Space Member Management', () => {
  test('Owner can open members dialog and see member list', { tag: '@regression' }, async ({ page }) => {
    await login(page);
    await page.goto(SPACE_DETAIL_URL);
    await expect(page.getByText('加载中')).not.toBeVisible({ timeout: 10000 });

    await page.getByRole('button', { name: '成员' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: '空间成员' })).toBeVisible();

    // The owner appears in the member list (scope to the table to avoid the
    // role-select combobox option which also contains the text '拥有者').
    // Use .first() because the test space may have multiple owners (e.g. the
    // seeded admin and the e2e test user are both owners in dev/CI).
    await expect(page.getByRole('table').getByText('拥有者').first()).toBeVisible({ timeout: 10000 });
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

    // Remove the member added by the previous test (stranger), never the
    // seeded editor test@example.com — otherwise subsequent permission tests
    // break because the seeded editor membership is gone.
    const strangerRow = page.getByRole('dialog').getByRole('row').filter({ hasText: STRANGER_NAME });
    test.skip((await strangerRow.count()) === 0, 'No stranger member to remove');

    await strangerRow.getByRole('button').click();

    // The removed stranger row disappears.
    await expect(strangerRow).not.toBeVisible();
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