// spec: specs/eap-test-plan.md
import { test, expect } from '@playwright/test';

test.describe('Value Stream Management - CRUD Operations', () => {
  test.beforeEach(async ({ page }) => {
    // Login before each test
    await page.goto('/login');
    await page.getByRole('textbox', { name: '邮箱' }).fill('test@example.com');
    await page.getByRole('textbox', { name: '密码' }).fill('testpassword123');
    await page.getByRole('button', { name: '登录' }).click();
    await expect(page).toHaveURL('/spaces/00000000-0000-0000-0000-000000000010/architectures/value-streams');
  });

  test('Happy Path - Create Value Stream', async ({ page }) => {
    // Click "新建价值流" button
    await page.getByRole('button', { name: '新建价值流' }).click();
    
    // Verify create dialog opens
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: /新建价值流|创建价值流/ })).toBeVisible();
    
    // Fill in form
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('测试价值流');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('这是一个测试价值流');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    // Select status (assuming it's a select/dropdown)
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.fill('active');
    
    // Select importance (assuming it's a select/dropdown)
    const importanceField = page.getByRole('combobox', { name: /重要性|Importance/ }).or(page.getByRole('textbox', { name: /重要性|Importance/ }));
    await importanceField.fill('high');
    
    // Click "保存" button
    await page.getByRole('button', { name: /保存|Save/ }).click();
    
    // Verify dialog closes
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Verify new value stream appears in table
    await expect(page.getByText('测试价值流')).toBeVisible({ timeout: 10000 });
    
    // Verify table shows correct data
    const row = page.locator('tr').filter({ hasText: '测试价值流' });
    await expect(row).toBeVisible();
    await expect(row.getByText('v1.0')).toBeVisible();
    await expect(row.getByText('active')).toBeVisible();
    
    // Note: Pagination count verification would require checking the table structure
    // For now, we verify the item appears in the table
  });

  test('Happy Path - Edit Value Stream', async ({ page }) => {
    // First, create a value stream to edit
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('原始名称');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('原始描述');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.fill('active');
    
    const importanceField = page.getByRole('combobox', { name: /重要性|Importance/ }).or(page.getByRole('textbox', { name: /重要性|Importance/ }));
    await importanceField.fill('medium');
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the created value stream and click edit button
    const row = page.locator('tr').filter({ hasText: '原始名称' });
    await expect(row).toBeVisible();
    
    // Click edit (pencil) button
    await row.getByRole('button').filter({ has: page.locator('svg[data-icon="pencil"]') }).click();
    
    // Verify edit dialog opens with pre-filled data
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: /编辑|Edit/ })).toBeVisible();
    
    // Verify form fields have existing data
    const nameField = page.getByRole('textbox', { name: /名称|Name/ });
    await expect(nameField).toHaveValue('原始名称');
    
    const descField = page.getByRole('textbox', { name: /描述|Description/ });
    await expect(descField).toHaveValue('原始描述');
    
    // Modify fields
    await nameField.fill('Updated Name');
    await descField.fill('Updated Description');
    
    // Click "保存" button
    await page.getByRole('button', { name: /保存|Save/ }).click();
    
    // Verify dialog closes
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Verify table shows updated data
    await expect(page.getByText('Updated Name')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('Updated Description')).toBeVisible();
    
    // Verify other fields unchanged
    await expect(page.getByText('v1.0')).toBeVisible();
    await expect(page.getByText('active')).toBeVisible();
  });

  test('Happy Path - Delete Value Stream', async ({ page }) => {
    // First, create a value stream to delete
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('待删除价值流');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('这个将被删除');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.fill('active');
    
    const importanceField = page.getByRole('combobox', { name: /重要性|Importance/ }).or(page.getByRole('textbox', { name: /重要性|Importance/ }));
    await importanceField.fill('low');
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the created value stream
    const row = page.locator('tr').filter({ hasText: '待删除价值流' });
    await expect(row).toBeVisible();
    
    // Click delete (trash) button
    await row.getByRole('button').filter({ has: page.locator('svg[data-icon="trash-2"]') }).click();
    
    // Verify delete confirmation dialog opens
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByText(/确认删除|Confirm delete/)).toBeVisible();
    
    // Click "确认" button
    await page.getByRole('button', { name: /确认|Confirm/ }).click();
    
    // Verify dialog closes
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Verify value stream removed from table
    await expect(page.getByText('待删除价值流')).not.toBeVisible({ timeout: 10000 });
  });

  test('Edge Case - Create Value Stream Validation', async ({ page }) => {
    await page.getByRole('button', { name: '新建价值流' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Test Case 1: Empty name field
    await page.getByRole('textbox', { name: /名称|Name/ }).clear();
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('描述');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    // Try to submit with empty name
    await page.getByRole('button', { name: /保存|Save/ }).click();
    
    // Should show validation error or prevent submission
    // Check for validation message or that dialog is still open
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Test Case 2: Invalid version format (if validation exists)
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('测试名称');
    await page.getByRole('textbox', { name: /版本|Version/ }).clear();
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('invalid version');
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    
    // Should show validation error
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Close dialog
    await page.getByRole('button', { name: /取消|Cancel/ }).or(page.locator('button[aria-label="Close"]')).click();
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });

  test('Edge Case - Delete Confirmation Cancel', async ({ page }) => {
    // Create a value stream
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('测试取消删除');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('测试取消删除描述');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.fill('active');
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the created value stream
    const row = page.locator('tr').filter({ hasText: '测试取消删除' });
    await expect(row).toBeVisible();
    
    // Click delete (trash) button
    await row.getByRole('button').filter({ has: page.locator('svg[data-icon="trash-2"]') }).click();
    
    // Verify delete confirmation dialog opens
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Click "取消" or close dialog
    const cancelButton = page.getByRole('button', { name: /取消|Cancel/ });
    if (await cancelButton.isVisible()) {
      await cancelButton.click();
    } else {
      // If no cancel button, close the dialog
      await page.locator('button[aria-label="Close"]').click();
    }
    
    // Verify dialog closes
    await expect(page.getByRole('dialog')).not.toBeVisible();
    
    // Verify value stream still in table
    await expect(page.getByText('测试取消删除')).toBeVisible();
  });

  test('View Value Stream Details', async ({ page }) => {
    // First, create a value stream to view
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('查看详情测试');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('这是一个用于查看详情的测试价值流');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.fill('active');
    
    const importanceField = page.getByRole('combobox', { name: /重要性|Importance/ }).or(page.getByRole('textbox', { name: /重要性|Importance/ }));
    await importanceField.fill('high');
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the created value stream and click "查看" button
    const row = page.locator('tr').filter({ hasText: '查看详情测试' });
    await expect(row).toBeVisible();
    
    // Click "查看" button
    await row.getByRole('button', { name: '查看' }).click();
    
    // Verify detail page loads
    await expect(page).toHaveURL(/\/architectures\/value-streams\/.+/);
    
    // Verify all value stream data displayed
    await expect(page.getByText('查看详情测试')).toBeVisible();
    await expect(page.getByText('这是一个用于查看详情的测试价值流')).toBeVisible();
    await expect(page.getByText('v1.0')).toBeVisible();
    await expect(page.getByText('active')).toBeVisible();
    
    // Look for "返回列表" button and click it
    const backButton = page.getByRole('button', { name: '返回列表' });
    if (await backButton.isVisible()) {
      await backButton.click();
      await expect(page).toHaveURL('/spaces/00000000-0000-0000-0000-000000000010/architectures/value-streams');
    } else {
      // Use browser back if no back button
      await page.goBack();
      await expect(page).toHaveURL('/spaces/00000000-0000-0000-0000-000000000010/architectures/value-streams');
    }
  });
});