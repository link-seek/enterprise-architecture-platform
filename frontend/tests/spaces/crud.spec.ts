// spec: specs/eap-test-plan.md
import { test, expect } from '../helpers/graphql-aware';
import { loginAsAdmin } from '../helpers/auth';

test.describe('Spaces - CRUD Main Flow', () => {
  test.beforeEach(async ({ page }) => {
    // Use admin login to bypass the 3-space quota for non-admin users.
    // Previous test runs leave archived spaces that count toward the quota.
    // Credentials are env-driven so production smoke tests resolve the real
    // seed admin (SMOKE_TEST_* / APP_SEED_ADMIN_* secrets) instead of the
    // local/CI default admin@test.com / admin123456.
    await loginAsAdmin(page);

    // Go to the spaces list page.
    await page.goto('/spaces');
    await expect(page.getByRole('heading', { name: '所有空间' })).toBeVisible();
  });

  test('Happy Path - Create, edit, and archive a space', { tag: ['@smoke', '@regression'] }, async ({ page }) => {
    const spaceName = `E2E空间_${Date.now()}`;
    const editedName = `${spaceName}_已编辑`;

    // ── Create ────────────────────────────────────────────────────────────
    await page.getByRole('button', { name: '创建空间' }).click();

    // Create dialog opens.
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: '创建空间' })).toBeVisible();

    // Fill name + description.
    await page.getByLabel('名称').fill(spaceName);
    await page.getByLabel('描述').fill('E2E 自动化测试空间');

    // Submit.
    await page.getByRole('button', { name: '创建', exact: true }).click();

    // Dialog closes and the new space appears in the list.
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    await expect(page.getByText(spaceName)).toBeVisible({ timeout: 10000 });

    // ── Edit ──────────────────────────────────────────────────────────────
    // Enter the newly created space.
    await page.getByRole('link', { name: new RegExp(spaceName) }).click();
    await expect(page).toHaveURL(/\/spaces\/[0-9a-f-]{36}$/);

    // Open the edit dialog.
    await page.getByRole('button', { name: '编辑' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: '编辑空间' })).toBeVisible();

    // Change the name and save.
    const nameInput = page.getByLabel('名称');
    await nameInput.fill('');
    await nameInput.fill(editedName);
    await page.getByRole('button', { name: '保存', exact: true }).click();

    // Dialog closes.
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });

    // Return to the spaces list and verify the updated name.
    await page.goto('/spaces');
    await expect(page.getByText(editedName)).toBeVisible({ timeout: 10000 });

    // ── Archive ───────────────────────────────────────────────────────────
    // Enter the space again to archive it.
    await page.getByRole('link', { name: new RegExp(editedName) }).click();
    await expect(page).toHaveURL(/\/spaces\/[0-9a-f-]{36}$/);

    // Click 归档 to open the confirmation dialog, then confirm.
    await page.getByRole('button', { name: '归档' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: '确认归档' })).toBeVisible();
    await page.getByRole('dialog').getByRole('button', { name: '归档' }).click();

    // Archiving navigates back to the spaces list.
    await expect(page).toHaveURL('/spaces', { timeout: 10000 });

    // The archived space no longer appears in the list (GET_SPACES filters
    // deletedAt is_null: true).
    await expect(page.getByText(editedName)).not.toBeVisible({ timeout: 10000 });
  });
});