// spec: specs/eap-test-plan.md
// 能力归档（discussions#752 D1/D2）：Active → Archived 直接边；归档后列表仍显示 +
// archived 徽标，归档按钮消失；清理时删除自证。
import { test, expect } from '../helpers/graphql-aware';
import { login, SPACE_BASE } from '../helpers/auth';

test.describe('Business Capabilities Management - Archive', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.getByRole('link', { name: '业务能力', exact: true }).click();
    await expect(page).toHaveURL(`${SPACE_BASE}/capabilities`);
  });

  test('Happy Path - Archive Business Capability', { tag: ['@smoke', '@regression'] }, async ({ page }) => {
    const name = `待归档能力_${Date.now()}`;
    await page.getByRole('button', { name: /新建能力|新建业务能力|New Business Capability/ }).click();
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(name);
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('这个将被归档');
    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });

    const row = page.locator('tr').filter({ hasText: name });
    await expect(row).toBeVisible();
    await expect(row.getByText('active')).toBeVisible();

    // 归档按钮是带 aria-label 的图标按钮，直接按名定位。
    const archiveButton = row.getByRole('button', { name: '归档' });
    await expect(archiveButton).toBeVisible({ timeout: 10000 });
    await archiveButton.click();

    // 等 refetch 回写新状态再 reload，避免取消在途 mutation。
    await expect(row.getByText('archived')).toBeVisible({ timeout: 10000 });

    await page.reload();
    const updatedRow = page.locator('tr').filter({ hasText: name });
    await expect(updatedRow.getByText('archived')).toBeVisible({ timeout: 10000 });
    await expect(updatedRow.getByRole('button', { name: '归档' })).not.toBeVisible();

    // 自删自证：归档行仍可删除
    await updatedRow.getByRole('button').filter({ has: page.locator('svg[class*="lucide-trash-2"]') }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await page.getByRole('button', { name: /确认|删除|Confirm|Delete/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    await page.reload();
    await expect(page.getByText(name, { exact: true })).not.toBeVisible({ timeout: 10000 });
  });
});
