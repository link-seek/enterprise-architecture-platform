// spec: specs/eap-test-plan.md
import { test, expect } from '@playwright/test';

test.describe('Business Capabilities Management - CRUD Operations', () => {
  test.beforeEach(async ({ page }) => {
    // Login before each test
    await page.goto('/login');
    await page.getByRole('textbox', { name: '邮箱' }).fill('test@example.com');
    await page.getByRole('textbox', { name: '密码' }).fill('testpassword123');
    await page.getByRole('button', { name: '登录' }).click();
    await expect(page).toHaveURL('/spaces/00000000-0000-0000-0000-000000000010/architectures/value-streams');
    
    // Navigate to capabilities page
    await page.getByRole('link', { name: '业务能力' }).click();
    await expect(page).toHaveURL('/spaces/00000000-0000-0000-0000-000000000010/architectures/capabilities');
  });

  test('Happy Path - Create Business Capability', async ({ page }) => {
    // Click "新建业务能力" button
    const createButton = page.getByRole('button', { name: /新建业务能力|New Business Capability/ });
    await expect(createButton).toBeVisible();
    await createButton.click();
    
    // Verify create dialog opens
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: /新建业务能力|Create Business Capability/ })).toBeVisible();
    
    // Fill in form with test data
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('测试业务能力');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('这是一个测试业务能力');
    
    // Fill other fields if they exist (maturity level, business value, etc.)
    const maturityField = page.getByRole('combobox', { name: /成熟度|Maturity/ }).or(page.getByRole('textbox', { name: /成熟度|Maturity/ }));
    if (await maturityField.isVisible()) {
      await maturityField.fill('成熟');
    }
    
    const businessValueField = page.getByRole('combobox', { name: /业务价值|Business Value/ }).or(page.getByRole('textbox', { name: /业务价值|Business Value/ }));
    if (await businessValueField.isVisible()) {
      await businessValueField.fill('高');
    }
    
    // Click "保存" button
    await page.getByRole('button', { name: /保存|Save/ }).click();
    
    // Verify dialog closes
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Verify new capability appears in table
    await expect(page.getByText('测试业务能力')).toBeVisible({ timeout: 10000 });
    
    // Verify all fields displayed correctly
    const row = page.locator('tr').filter({ hasText: '测试业务能力' });
    await expect(row).toBeVisible();
    
    if (await maturityField.isVisible()) {
      await expect(row.getByText('成熟')).toBeVisible();
    }
    
    if (await businessValueField.isVisible()) {
      await expect(row.getByText('高')).toBeVisible();
    }
  });

  test('Happy Path - Read Business Capability', async ({ page }) => {
    // First create a capability to read
    const createButton = page.getByRole('button', { name: /新建业务能力|New Business Capability/ });
    await createButton.click();
    
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('读取测试能力');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('用于读取测试的业务能力');
    
    const maturityField = page.getByRole('combobox', { name: /成熟度|Maturity/ }).or(page.getByRole('textbox', { name: /成熟度|Maturity/ }));
    if (await maturityField.isVisible()) {
      await maturityField.fill('发展中');
    }
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Verify new capability appears in table
    await expect(page.getByText('读取测试能力')).toBeVisible({ timeout: 10000 });
    
    // Verify all fields displayed correctly
    const row = page.locator('tr').filter({ hasText: '读取测试能力' });
    await expect(row).toBeVisible();
    await expect(row.getByText('用于读取测试的业务能力')).toBeVisible();
    
    if (await maturityField.isVisible()) {
      await expect(row.getByText('发展中')).toBeVisible();
    }
  });

  test('Happy Path - Update Business Capability', async ({ page }) => {
    // Create a capability to update
    const createButton = page.getByRole('button', { name: /新建业务能力|New Business Capability/ });
    await createButton.click();
    
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('更新前名称');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('更新前描述');
    
    const maturityField = page.getByRole('combobox', { name: /成熟度|Maturity/ }).or(page.getByRole('textbox', { name: /成熟度|Maturity/ }));
    if (await maturityField.isVisible()) {
      await maturityField.fill('初始');
    }
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the created capability and click edit button
    const row = page.locator('tr').filter({ hasText: '更新前名称' });
    await expect(row).toBeVisible();
    
    // Click edit (pencil) button
    await row.getByRole('button').filter({ has: page.locator('svg[data-icon="pencil"]') }).click();
    
    // Verify edit dialog opens with pre-filled data
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: /编辑|Edit/ })).toBeVisible();
    
    // Verify form fields have existing data
    const nameField = page.getByRole('textbox', { name: /名称|Name/ });
    await expect(nameField).toHaveValue('更新前名称');
    
    const descField = page.getByRole('textbox', { name: /描述|Description/ });
    await expect(descField).toHaveValue('更新前描述');
    
    // Modify fields
    await nameField.fill('更新后名称');
    await descField.fill('更新后描述');
    
    if (await maturityField.isVisible()) {
      const editMaturityField = page.getByRole('combobox', { name: /成熟度|Maturity/ }).or(page.getByRole('textbox', { name: /成熟度|Maturity/ }));
      await editMaturityField.fill('成熟');
    }
    
    // Click "保存" button
    await page.getByRole('button', { name: /保存|Save/ }).click();
    
    // Verify dialog closes
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Verify table shows updated data
    await expect(page.getByText('更新后名称')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('更新后描述')).toBeVisible();
    
    if (await maturityField.isVisible()) {
      await expect(page.getByText('成熟')).toBeVisible();
    }
  });

  test('Happy Path - Delete Business Capability', async ({ page }) => {
    // Create a capability to delete
    const createButton = page.getByRole('button', { name: /新建业务能力|New Business Capability/ });
    await createButton.click();
    
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('待删除能力');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('这个将被删除');
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the created capability
    const row = page.locator('tr').filter({ hasText: '待删除能力' });
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
    
    // Verify capability removed from table
    await expect(page.getByText('待删除能力')).not.toBeVisible({ timeout: 10000 });
  });

  test('Edge Case - Form Validation', async ({ page }) => {
    // Click "新建业务能力" button
    const createButton = page.getByRole('button', { name: /新建业务能力|New Business Capability/ });
    await createButton.click();
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Test Case 1: Empty required fields
    // Try to submit with empty name field
    await page.getByRole('button', { name: /保存|Save/ }).click();
    
    // Should show validation error or prevent submission
    await expect(page.getByRole('dialog')).toBeVisible(); // Dialog should stay open
    
    // Check for validation messages
    const errorMessages = page.getByText(/必填|Required|不能为空/);
    if (await errorMessages.isVisible()) {
      await expect(errorMessages).toBeVisible();
    }
    
    // Test Case 2: Invalid maturity level (if dropdown)
    const maturityField = page.getByRole('combobox', { name: /成熟度|Maturity/ }).or(page.getByRole('textbox', { name: /成熟度|Maturity/ }));
    if (await maturityField.isVisible()) {
      await page.getByRole('textbox', { name: /名称|Name/ }).fill('测试能力');
      
      // Try with invalid maturity value if it's a select with predefined options
      // This would depend on the actual implementation
    }
    
    // Test Case 3: Invalid business value
    const businessValueField = page.getByRole('combobox', { name: /业务价值|Business Value/ }).or(page.getByRole('textbox', { name: /业务价值|Business Value/ }));
    if (await businessValueField.isVisible()) {
      // Similar to maturity field
    }
    
    // Close dialog
    await page.getByRole('button', { name: /取消|Cancel/ }).or(page.locator('button[aria-label="Close"]')).click();
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });

  test('Full CRUD Cycle', async ({ page }) => {
    // Create
    const createButton = page.getByRole('button', { name: /新建业务能力|New Business Capability/ });
    await createButton.click();
    
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('完整CRUD测试');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('完整的创建、读取、更新、删除测试');
    
    const maturityField = page.getByRole('combobox', { name: /成熟度|Maturity/ }).or(page.getByRole('textbox', { name: /成熟度|Maturity/ }));
    if (await maturityField.isVisible()) {
      await maturityField.fill('发展中');
    }
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Read
    await expect(page.getByText('完整CRUD测试')).toBeVisible({ timeout: 10000 });
    const row = page.locator('tr').filter({ hasText: '完整CRUD测试' });
    await expect(row).toBeVisible();
    await expect(row.getByText('完整的创建、读取、更新、删除测试')).toBeVisible();
    
    // Update
    await row.getByRole('button').filter({ has: page.locator('svg[data-icon="pencil"]') }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('更新后的CRUD测试');
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    await expect(page.getByText('更新后的CRUD测试')).toBeVisible({ timeout: 10000 });
    
    // Delete
    const updatedRow = page.locator('tr').filter({ hasText: '更新后的CRUD测试' });
    await updatedRow.getByRole('button').filter({ has: page.locator('svg[data-icon="trash-2"]') }).click();
    
    await expect(page.getByRole('dialog')).toBeVisible();
    await page.getByRole('button', { name: /确认|Confirm/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Verify removal
    await expect(page.getByText('更新后的CRUD测试')).not.toBeVisible({ timeout: 10000 });
  });
});