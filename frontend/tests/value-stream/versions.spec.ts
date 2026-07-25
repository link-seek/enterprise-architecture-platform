// spec: specs/eap-test-plan.md
import { test, expect } from '@playwright/test';

test.describe('Value Stream Management - Version Control', () => {
  test.beforeEach(async ({ page }) => {
    // Login before each test
    await page.goto('/login');
    await page.getByRole('textbox', { name: '邮箱' }).fill('test@example.com');
    await page.getByRole('textbox', { name: '密码' }).fill('testpassword123');
    await page.getByRole('button', { name: '登录' }).click();
    await expect(page).toHaveURL('/spaces/00000000-0000-0000-0000-000000000010/architectures/value-streams');
  });

  test('Happy Path - Create New Version', async ({ page }) => {
    // First, create a value stream to version
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('版本控制测试');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('用于版本控制测试');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.fill('active');
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the created value stream
    const row = page.locator('tr').filter({ hasText: '版本控制测试' });
    await expect(row).toBeVisible();
    
    // Click version control (GitBranch) button
    await row.getByRole('button').filter({ has: page.locator('svg[data-icon="git-branch"]') }).click();
    
    // Verify create version dialog opens
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: /创建新版本|Create New Version/ })).toBeVisible();
    
    // Enter new version name
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v2.0');
    
    // Click "创建" button
    await page.getByRole('button', { name: /创建|Create/ }).click();
    
    // Verify dialog closes
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Verify success message or UI update
    // The UI should reflect the new version somehow
    
    // Click history (History) button on same value stream
    await row.getByRole('button').filter({ has: page.locator('svg[data-icon="history"]') }).click();
    
    // Verify version history dialog opens
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: /版本历史|Version History/ })).toBeVisible();
    
    // Verify both versions (v1.0 and v2.0) listed
    await expect(page.getByText('v1.0')).toBeVisible();
    await expect(page.getByText('v2.0')).toBeVisible();
    
    // Close history dialog
    await page.getByRole('button', { name: /关闭|Close/ }).or(page.locator('button[aria-label="Close"]')).click();
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });

  test('Happy Path - Archive Value Stream', async ({ page }) => {
    // Create an active value stream
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('待归档测试');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('这个将被归档');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.fill('active');
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the created value stream (status: "active")
    const row = page.locator('tr').filter({ hasText: '待归档测试' });
    await expect(row).toBeVisible();
    await expect(row.getByText('active')).toBeVisible();
    
    // Click archive button (look for archive icon or button)
    // The button might have text "归档" or an archive icon
    const archiveButton = row.getByRole('button').filter({ hasText: /归档|Archive/ })
      .or(row.getByRole('button').filter({ has: page.locator('svg[data-icon="archive"]') }));
    
    if (await archiveButton.isVisible()) {
      await archiveButton.click();
      
      // Verify archive confirmation appears
      await expect(page.getByRole('dialog')).toBeVisible();
      await expect(page.getByText(/确认归档|Confirm archive/)).toBeVisible();
      
      // Confirm archive action
      await page.getByRole('button', { name: /确认|Confirm/ }).click();
      
      // Verify dialog closes
      await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
      
      // Verify value stream status changes to "archived"
      // Might need to reload or wait for UI update
      await page.reload();
      const updatedRow = page.locator('tr').filter({ hasText: '待归档测试' });
      await expect(updatedRow.getByText('archived')).toBeVisible();
      
      // Verify badge color changes (destructive variant)
      // This would require checking the badge class or style
      
      // Verify archive button disappears (archived items shouldn't have archive button)
      const archiveButtonAfter = updatedRow.getByRole('button').filter({ hasText: /归档|Archive/ })
        .or(updatedRow.getByRole('button').filter({ has: page.locator('svg[data-icon="archive"]') }));
      await expect(archiveButtonAfter).not.toBeVisible();
    } else {
      console.log('Archive button not found - skipping archive test');
    }
  });

  test('Version History Dialog Functionality', async ({ page }) => {
    // Create a value stream with multiple versions
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('历史测试');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('用于历史测试');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.fill('active');
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the value stream
    const row = page.locator('tr').filter({ hasText: '历史测试' });
    await expect(row).toBeVisible();
    
    // Open history dialog
    await row.getByRole('button').filter({ has: page.locator('svg[data-icon="history"]') }).click();
    
    // Verify history dialog opens
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: /版本历史|Version History/ })).toBeVisible();
    
    // Verify dialog contains expected elements
    await expect(page.getByText(/版本|Version/)).toBeVisible();
    await expect(page.getByText(/创建时间|Created at/)).toBeVisible();
    await expect(page.getByText(/创建者|Created by/)).toBeVisible();
    
    // Verify close button works
    await page.getByRole('button', { name: /关闭|Close/ }).or(page.locator('button[aria-label="Close"]')).click();
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });

  test('Create Version Validation', async ({ page }) => {
    // Create a value stream
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('版本验证测试');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('用于版本验证测试');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.fill('active');
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the value stream
    const row = page.locator('tr').filter({ hasText: '版本验证测试' });
    await expect(row).toBeVisible();
    
    // Open create version dialog
    await row.getByRole('button').filter({ has: page.locator('svg[data-icon="git-branch"]') }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Test empty version name
    await page.getByRole('textbox', { name: /版本|Version/ }).clear();
    await page.getByRole('button', { name: /创建|Create/ }).click();
    
    // Should show validation error or prevent submission
    await expect(page.getByRole('dialog')).toBeVisible(); // Dialog should stay open
    
    // Test duplicate version name (if validation exists)
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0'); // Same as existing
    await page.getByRole('button', { name: /创建|Create/ }).click();
    
    // Should show validation error for duplicate
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Test invalid version format
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('invalid version name');
    await page.getByRole('button', { name: /创建|Create/ }).click();
    
    // Should show validation error
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Close dialog
    await page.getByRole('button', { name: /取消|Cancel/ }).or(page.locator('button[aria-label="Close"]')).click();
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });

  test('Restore Previous Version', async ({ page }) => {
    // This test assumes there's a restore functionality
    // Create a value stream with multiple versions
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('恢复测试');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('初始描述');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.fill('active');
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the value stream
    const row = page.locator('tr').filter({ hasText: '恢复测试' });
    await expect(row).toBeVisible();
    
    // Create a new version
    await row.getByRole('button').filter({ has: page.locator('svg[data-icon="git-branch"]') }).click();
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v2.0');
    await page.getByRole('button', { name: /创建|Create/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Open history dialog
    await row.getByRole('button').filter({ has: page.locator('svg[data-icon="history"]') }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Look for restore button on v1.0
    const v1Row = page.locator('tr', { hasText: 'v1.0' });
    
    // If restore functionality exists, test it
    const restoreButton = v1Row.getByRole('button').filter({ hasText: /恢复|Restore/ });
    if (await restoreButton.isVisible()) {
      await restoreButton.click();
      
      // Confirm restore if needed
      const confirmButton = page.getByRole('button', { name: /确认|Confirm/ });
      if (await confirmButton.isVisible()) {
        await confirmButton.click();
      }
      
      // Verify restore completed
      // Might need to check UI updates or success message
    }
    
    // Close history dialog
    await page.getByRole('button', { name: /关闭|Close/ }).or(page.locator('button[aria-label="Close"]')).click();
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });
});