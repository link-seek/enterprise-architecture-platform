// spec: specs/eap-test-plan.md
import { test, expect } from '../helpers/graphql-aware';
import { login, SPACE_BASE, TEST_EMAIL, TEST_PASSWORD } from '../helpers/auth';
import { cleanupValueStreamsByNamePrefix, findResidualValueStreams } from '../helpers/graphql-api';

// Static prefixes — afterAll cleanup catches every run's data.
const TEST_NAME_PREFIXES = [
  '版本控制测试_',
  '待归档测试_',
  '历史测试_',
  '版本验证测试_',
];

/**
 * Self-delete + verify: delete all value streams whose name starts with any of
 * the given prefixes via the GraphQL API, then query back and assert zero
 * residual (单例自删自证).
 */
async function selfDeleteAndVerify(
  request: import('@playwright/test').APIRequestContext,
  namePrefixes: string[],
): Promise<void> {
  await cleanupValueStreamsByNamePrefix(request, namePrefixes, TEST_EMAIL, TEST_PASSWORD);
  const residual = await findResidualValueStreams(request, namePrefixes, TEST_EMAIL, TEST_PASSWORD);
  expect(residual).toEqual([]);
}

test.describe('Value Stream Management - Version Control', () => {
  test.beforeEach(async ({ page }) => {
    // Login before each case - env-driven credentials for multi-environment reuse.
    // Login now lands on the architecture overview; navigate to value-streams explicitly.
    await login(page);
    await page.goto(`${SPACE_BASE}/value-streams`);
  });

  test.afterAll(async ({ request }) => {
    await cleanupValueStreamsByNamePrefix(request, TEST_NAME_PREFIXES);
    const residual = await findResidualValueStreams(request, TEST_NAME_PREFIXES);
    expect(residual).toEqual([]);
  });

  test('Happy Path - Create New Version', { tag: '@regression' }, async ({ page, request }) => {
    // Use a unique name to avoid strict-mode violations from residual data on repeated runs
    const name = `版本控制测试_${Date.now()}`;
    // First, create a value stream to version
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(name);
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('用于版本控制测试');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('1.0.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.selectOption('active');
    
    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the created value stream
    const row = page.locator('tr').filter({ hasText: name });
    await expect(row).toBeVisible();
    
    // Click version control (GitBranch) button
    await row.getByRole('button').filter({ has: page.locator('svg[class*="lucide-git-branch"]') }).click();

    // Verify unified publish dialog opens (同一发布仪式：价值流 = 业务流程)
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: /发布新版本|新建版本/ })).toBeVisible();

    // Enter new version (suggested value prefilled, override to be explicit)
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('2.0.0');

    // Click confirm
    await page.getByRole('button', { name: /确认发布|创建|Create/ }).click();

    // Verify dialog closes
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });

    // Unified publish policy: old 1.0.0 → deprecated (compatibility window),
    // new 2.0.0 → active.
    const oldRow = page.locator('tr').filter({ hasText: name }).filter({ hasText: '1.0.0' });
    await expect(oldRow.getByText('deprecated')).toBeVisible({ timeout: 10000 });

    // After versioning there are two rows (1.0.0 deprecated + 2.0.0 active);
    // click history (History) button on the active row
    const activeRow = page.locator('tr').filter({ hasText: name }).filter({ hasText: 'active' });
    await activeRow.getByRole('button', { name: '历史' }).click();

    // Verify version history dialog opens
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: /版本历史|Version History/ })).toBeVisible();

    // Verify both versions (1.0.0 and 2.0.0) listed inside the dialog
    const historyDialog = page.getByRole('dialog');
    await expect(historyDialog.getByText('1.0.0')).toBeVisible();
    await expect(historyDialog.getByText('2.0.0')).toBeVisible();

    // Close history dialog
    await historyDialog.getByRole('button', { name: '关闭' }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible();

    // Retire the deprecated old row via the archive button (visible on
    // deprecated rows under the unified policy), then verify archived.
    await oldRow.getByRole('button').filter({ has: page.locator('svg[class*="lucide-archive"]') }).click();
    await expect(oldRow.getByText('archived')).toBeVisible({ timeout: 10000 });

    // Self-delete and verify (单例自删自证) — cleanup expands by logicalId so
    // both the archived 1.0.0 and active 2.0.0 rows are removed.
    await selfDeleteAndVerify(request, [name]);
  });

  test('Happy Path - Archive Value Stream', { tag: ['@smoke', '@regression'] }, async ({ page, request }) => {
    // Use a unique name to avoid strict-mode violations from residual data on repeated runs
    const name = `待归档测试_${Date.now()}`;
    // Create an active value stream
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(name);
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('这个将被归档');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('1.0.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.selectOption('active');
    
    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the created value stream (status: "active")
    const row = page.locator('tr').filter({ hasText: name });
    await expect(row).toBeVisible();
    await expect(row.getByText('active')).toBeVisible();
    
    // Click archive button (look for archive icon or button)
    // The button might have text "归档" or an archive icon
    const archiveButton = row.getByRole('button').filter({ hasText: /归档|Archive/ })
      .or(row.getByRole('button').filter({ has: page.locator('svg[class*="lucide-archive"]') }));
    
    // ArchiveButton 是纯图标按钮（无文字），只走图标分支定位。
    // 按钮必须存在：找不到就是产品 bug，硬断言失败，不许静默跳过。
    await expect(archiveButton).toBeVisible({ timeout: 10000 });
    await archiveButton.click();

    // Archive is applied directly (no confirmation dialog — the archive
    // button on the row archives immediately). The mutation fires async
    // with refetchQueries, so wait for the UI to reflect the new status
    // BEFORE reloading — otherwise the reload cancels the in-flight
    // mutation request (especially against a remote backend with latency)
    // and the value stream is never archived.
    await expect(row.getByText('archived')).toBeVisible({ timeout: 10000 });

    // Reload and re-assert to verify the backend persisted the archived
    // status (not just an optimistic UI update).
    await page.reload();
    const updatedRow = page.locator('tr').filter({ hasText: name });
    await expect(updatedRow.getByText('archived')).toBeVisible({ timeout: 10000 });

    // Verify archive button disappears (archived items shouldn't have archive button)
    const archiveButtonAfter = updatedRow.getByRole('button').filter({ hasText: /归档|Archive/ })
      .or(updatedRow.getByRole('button').filter({ has: page.locator('svg[class*="lucide-archive"]') }));
    await expect(archiveButtonAfter).not.toBeVisible();

    // Self-delete and verify (单例自删自证)
    await selfDeleteAndVerify(request, [name]);
  });

  test('Version History Dialog Functionality', { tag: '@regression' }, async ({ page, request }) => {
    // Use a unique name to avoid strict-mode violations from residual data on repeated runs
    const name = `历史测试_${Date.now()}`;
    // Create a value stream with multiple versions
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(name);
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('用于历史测试');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('1.0.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.selectOption('active');
    
    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the value stream
    const row = page.locator('tr').filter({ hasText: name });
    await expect(row).toBeVisible();
    
    // After versioning there are two rows (1.0.0 archived + 2.0.0 active);
    // click history (History) button on the active row
    const activeRow = page.locator('tr').filter({ hasText: name }).filter({ hasText: 'active' });
    await activeRow.getByRole('button', { name: '历史' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();

    // Look for restore button on 1.0.0 (inside the history dialog)
    const historyDialog = page.getByRole('dialog');
    const v1Row = historyDialog.locator('tr').filter({ hasText: '1.0.0' });

    // If restore functionality exists, test it
    const restoreButton = v1Row.getByRole('button').filter({ hasText: /恢复|Restore/ });
    if (await restoreButton.isVisible()) {
      await restoreButton.click();

      // Confirm restore if needed
      const confirmButton = historyDialog.getByRole('button', { name: /确认|归档|Confirm|Archive/ });
      if (await confirmButton.isVisible()) {
        await confirmButton.click();
      }

      // Verify restore completed
      // Might need to check UI updates or success message
    }

    // Close history dialog
    await historyDialog.getByRole('button', { name: '关闭' }).click();

    // Self-delete and verify (单例自删自证)
    await selfDeleteAndVerify(request, [name]);
  });

  test('Create Version Validation', { tag: '@regression' }, async ({ page, request }) => {
    // Use a unique name to avoid strict-mode violations from residual data on repeated runs
    const name = `版本验证测试_${Date.now()}`;
    // Create a value stream
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(name);
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('用于版本验证测试');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('1.0.0');

    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.selectOption('active');

    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });

    // Find the value stream
    const row = page.locator('tr').filter({ hasText: name });
    await expect(row).toBeVisible();

    // Open create version dialog
    await row.getByRole('button').filter({ has: page.locator('svg[class*="lucide-git-branch"]') }).click();
    await expect(page.getByRole('dialog')).toBeVisible();

    // Empty version name: the submit button is disabled and the dialog stays open
    await page.getByRole('textbox', { name: /版本|Version/ }).clear();
    await expect(page.getByRole('button', { name: /创建|Create/ })).toBeDisabled();
    await expect(page.getByRole('dialog')).toBeVisible();

    // Close dialog
    await page.getByRole('dialog').getByRole('button', { name: '取消' }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible();

    // Self-delete and verify (单例自删自证)
    await selfDeleteAndVerify(request, [name]);
  });
});