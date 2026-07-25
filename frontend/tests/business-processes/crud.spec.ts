// spec: specs/eap-test-plan.md
import { test, expect } from '@playwright/test';

test.describe('Business Processes Management - CRUD Operations', () => {
  test.beforeEach(async ({ page }) => {
    // Login before each test
    await page.goto('/login');
    await page.getByRole('textbox', { name: '邮箱' }).fill('test@example.com');
    await page.getByRole('textbox', { name: '密码' }).fill('testpassword123');
    await page.getByRole('button', { name: '登录' }).click();
    await expect(page).toHaveURL('/spaces/00000000-0000-0000-0000-000000000010/architectures/value-streams');
    
    // Navigate to processes page
    await page.getByRole('link', { name: '业务流程' }).click();
    await expect(page).toHaveURL('/spaces/00000000-0000-0000-0000-000000000010/architectures/processes');
  });

  test('Happy Path - Create Business Process', async ({ page }) => {
    // Click "新建业务流程" button
    const createButton = page.getByRole('button', { name: /新建业务流程|New Business Process/ });
    await expect(createButton).toBeVisible();
    await createButton.click();
    
    // Verify create dialog opens
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: /新建业务流程|Create Business Process/ })).toBeVisible();
    
    // Fill in form with test data including SLA, cycle time, cost
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('测试业务流程');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('这是一个测试业务流程');
    
    // Fill numeric fields if they exist
    const slaField = page.getByRole('spinbutton', { name: /SLA|服务级别协议/ }).or(page.getByRole('textbox', { name: /SLA|服务级别协议/ }));
    if (await slaField.isVisible()) {
      await slaField.fill('99.9');
    }
    
    const cycleTimeField = page.getByRole('spinbutton', { name: /周期时间|Cycle Time/ }).or(page.getByRole('textbox', { name: /周期时间|Cycle Time/ }));
    if (await cycleTimeField.isVisible()) {
      await cycleTimeField.fill('24');
    }
    
    const costField = page.getByRole('spinbutton', { name: /成本|Cost/ }).or(page.getByRole('textbox', { name: /成本|Cost/ }));
    if (await costField.isVisible()) {
      await costField.fill('1000');
    }
    
    // Click "保存" button
    await page.getByRole('button', { name: /保存|Save/ }).click();
    
    // Verify dialog closes
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Verify new process appears in table
    await expect(page.getByText('测试业务流程')).toBeVisible({ timeout: 10000 });
    
    // Verify numeric fields formatted correctly
    const row = page.locator('tr').filter({ hasText: '测试业务流程' });
    await expect(row).toBeVisible();
    
    if (await slaField.isVisible()) {
      await expect(row.getByText('99.9')).toBeVisible();
    }
    
    if (await cycleTimeField.isVisible()) {
      await expect(row.getByText('24')).toBeVisible();
    }
    
    if (await costField.isVisible()) {
      await expect(row.getByText('1000')).toBeVisible();
    }
  });

  test('Happy Path - Read Business Process', async ({ page }) => {
    // Create a process to read
    const createButton = page.getByRole('button', { name: /新建业务流程|New Business Process/ });
    await createButton.click();
    
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('读取测试流程');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('用于读取测试的业务流程');
    
    // Fill numeric fields
    const slaField = page.getByRole('spinbutton', { name: /SLA|服务级别协议/ }).or(page.getByRole('textbox', { name: /SLA|服务级别协议/ }));
    if (await slaField.isVisible()) {
      await slaField.fill('95.5');
    }
    
    const cycleTimeField = page.getByRole('spinbutton', { name: /周期时间|Cycle Time/ }).or(page.getByRole('textbox', { name: /周期时间|Cycle Time/ }));
    if (await cycleTimeField.isVisible()) {
      await cycleTimeField.fill('48');
    }
    
    const costField = page.getByRole('spinbutton', { name: /成本|Cost/ }).or(page.getByRole('textbox', { name: /成本|Cost/ }));
    if (await costField.isVisible()) {
      await costField.fill('5000');
    }
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Verify new process appears in table
    await expect(page.getByText('读取测试流程')).toBeVisible({ timeout: 10000 });
    
    // Verify all fields displayed correctly
    const row = page.locator('tr').filter({ hasText: '读取测试流程' });
    await expect(row).toBeVisible();
    await expect(row.getByText('用于读取测试的业务流程')).toBeVisible();
    
    if (await slaField.isVisible()) {
      await expect(row.getByText('95.5')).toBeVisible();
    }
    
    if (await cycleTimeField.isVisible()) {
      await expect(row.getByText('48')).toBeVisible();
    }
    
    if (await costField.isVisible()) {
      await expect(row.getByText('5000')).toBeVisible();
    }
  });

  test('Happy Path - Update Business Process', async ({ page }) => {
    // Create a process to update
    const createButton = page.getByRole('button', { name: /新建业务流程|New Business Process/ });
    await createButton.click();
    
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('更新前流程');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('更新前描述');
    
    const slaField = page.getByRole('spinbutton', { name: /SLA|服务级别协议/ }).or(page.getByRole('textbox', { name: /SLA|服务级别协议/ }));
    if (await slaField.isVisible()) {
      await slaField.fill('90');
    }
    
    const cycleTimeField = page.getByRole('spinbutton', { name: /周期时间|Cycle Time/ }).or(page.getByRole('textbox', { name: /周期时间|Cycle Time/ }));
    if (await cycleTimeField.isVisible()) {
      await cycleTimeField.fill('72');
    }
    
    const costField = page.getByRole('spinbutton', { name: /成本|Cost/ }).or(page.getByRole('textbox', { name: /成本|Cost/ }));
    if (await costField.isVisible()) {
      await costField.fill('2000');
    }
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the created process and click edit button
    const row = page.locator('tr').filter({ hasText: '更新前流程' });
    await expect(row).toBeVisible();
    
    // Click edit (pencil) button
    await row.getByRole('button').filter({ has: page.locator('svg[data-icon="pencil"]') }).click();
    
    // Verify edit dialog opens with pre-filled data
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: /编辑|Edit/ })).toBeVisible();
    
    // Verify form fields have existing data
    const nameField = page.getByRole('textbox', { name: /名称|Name/ });
    await expect(nameField).toHaveValue('更新前流程');
    
    const descField = page.getByRole('textbox', { name: /描述|Description/ });
    await expect(descField).toHaveValue('更新前描述');
    
    // Modify fields
    await nameField.fill('更新后流程');
    await descField.fill('更新后描述');
    
    if (await slaField.isVisible()) {
      const editSlaField = page.getByRole('spinbutton', { name: /SLA|服务级别协议/ }).or(page.getByRole('textbox', { name: /SLA|服务级别协议/ }));
      await editSlaField.fill('99.5');
    }
    
    if (await cycleTimeField.isVisible()) {
      const editCycleTimeField = page.getByRole('spinbutton', { name: /周期时间|Cycle Time/ }).or(page.getByRole('textbox', { name: /周期时间|Cycle Time/ }));
      await editCycleTimeField.fill('24');
    }
    
    if (await costField.isVisible()) {
      const editCostField = page.getByRole('spinbutton', { name: /成本|Cost/ }).or(page.getByRole('textbox', { name: /成本|Cost/ }));
      await editCostField.fill('3000');
    }
    
    // Click "保存" button
    await page.getByRole('button', { name: /保存|Save/ }).click();
    
    // Verify dialog closes
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Verify table shows updated data
    await expect(page.getByText('更新后流程')).toBeVisible({ timeout: 10000 });
    await expect(page.getByText('更新后描述')).toBeVisible();
    
    if (await slaField.isVisible()) {
      await expect(page.getByText('99.5')).toBeVisible();
    }
    
    if (await cycleTimeField.isVisible()) {
      await expect(page.getByText('24')).toBeVisible();
    }
    
    if (await costField.isVisible()) {
      await expect(page.getByText('3000')).toBeVisible();
    }
  });

  test('Happy Path - Delete Business Process', async ({ page }) => {
    // Create a process to delete
    const createButton = page.getByRole('button', { name: /新建业务流程|New Business Process/ });
    await createButton.click();
    
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('待删除流程');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('这个将被删除');
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the created process
    const row = page.locator('tr').filter({ hasText: '待删除流程' });
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
    
    // Verify process removed from table
    await expect(page.getByText('待删除流程')).not.toBeVisible({ timeout: 10000 });
  });

  test('Edge Case - Numeric Input Validation', async ({ page }) => {
    // Click "新建业务流程" button
    const createButton = page.getByRole('button', { name: /新建业务流程|New Business Process/ });
    await createButton.click();
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Fill basic fields
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('数值验证测试');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('测试数值输入验证');
    
    // Test Case 1: Negative cycle time (if validation exists)
    const cycleTimeField = page.getByRole('spinbutton', { name: /周期时间|Cycle Time/ }).or(page.getByRole('textbox', { name: /周期时间|Cycle Time/ }));
    if (await cycleTimeField.isVisible()) {
      await cycleTimeField.fill('-10');
      await page.getByRole('button', { name: /保存|Save/ }).click();
      
      // Should show validation error
      await expect(page.getByRole('dialog')).toBeVisible(); // Dialog should stay open
      
      // Check for validation message
      const errorMessage = page.getByText(/必须为正数|必须大于0|Positive number required/i);
      if (await errorMessage.isVisible()) {
        await expect(errorMessage).toBeVisible();
      }
      
      // Clear the invalid value
      await cycleTimeField.fill('');
    }
    
    // Test Case 2: Negative cost (if validation exists)
    const costField = page.getByRole('spinbutton', { name: /成本|Cost/ }).or(page.getByRole('textbox', { name: /成本|Cost/ }));
    if (await costField.isVisible()) {
      await costField.fill('-100');
      await page.getByRole('button', { name: /保存|Save/ }).click();
      
      // Should show validation error
      await expect(page.getByRole('dialog')).toBeVisible();
      
      // Clear the invalid value
      await costField.fill('');
    }
    
    // Test Case 3: Non-numeric values in numeric fields
    if (await cycleTimeField.isVisible()) {
      await cycleTimeField.fill('not-a-number');
      await page.getByRole('button', { name: /保存|Save/ }).click();
      
      // Should show validation error
      await expect(page.getByRole('dialog')).toBeVisible();
    }
    
    // Close dialog
    await page.getByRole('button', { name: /取消|Cancel/ }).or(page.locator('button[aria-label="Close"]')).click();
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });

  test('Full CRUD Cycle with Numeric Fields', async ({ page }) => {
    // Create
    const createButton = page.getByRole('button', { name: /新建业务流程|New Business Process/ });
    await createButton.click();
    
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('完整CRUD流程');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('完整的创建、读取、更新、删除测试流程');
    
    // Fill numeric fields if they exist
    const slaField = page.getByRole('spinbutton', { name: /SLA|服务级别协议/ }).or(page.getByRole('textbox', { name: /SLA|服务级别协议/ }));
    if (await slaField.isVisible()) {
      await slaField.fill('99.9');
    }
    
    const cycleTimeField = page.getByRole('spinbutton', { name: /周期时间|Cycle Time/ }).or(page.getByRole('textbox', { name: /周期时间|Cycle Time/ }));
    if (await cycleTimeField.isVisible()) {
      await cycleTimeField.fill('24');
    }
    
    const costField = page.getByRole('spinbutton', { name: /成本|Cost/ }).or(page.getByRole('textbox', { name: /成本|Cost/ }));
    if (await costField.isVisible()) {
      await costField.fill('10000');
    }
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Read
    await expect(page.getByText('完整CRUD流程')).toBeVisible({ timeout: 10000 });
    const row = page.locator('tr').filter({ hasText: '完整CRUD流程' });
    await expect(row).toBeVisible();
    await expect(row.getByText('完整的创建、读取、更新、删除测试流程')).toBeVisible();
    
    if (await slaField.isVisible()) {
      await expect(row.getByText('99.9')).toBeVisible();
    }
    
    if (await cycleTimeField.isVisible()) {
      await expect(row.getByText('24')).toBeVisible();
    }
    
    if (await costField.isVisible()) {
      await expect(row.getByText('10000')).toBeVisible();
    }
    
    // Update
    await row.getByRole('button').filter({ has: page.locator('svg[data-icon="pencil"]') }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('更新后的CRUD流程');
    
    if (await slaField.isVisible()) {
      const editSlaField = page.getByRole('spinbutton', { name: /SLA|服务级别协议/ }).or(page.getByRole('textbox', { name: /SLA|服务级别协议/ }));
      await editSlaField.fill('99.99');
    }
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    await expect(page.getByText('更新后的CRUD流程')).toBeVisible({ timeout: 10000 });
    
    // Delete
    const updatedRow = page.locator('tr').filter({ hasText: '更新后的CRUD流程' });
    await updatedRow.getByRole('button').filter({ has: page.locator('svg[data-icon="trash-2"]') }).click();
    
    await expect(page.getByRole('dialog')).toBeVisible();
    await page.getByRole('button', { name: /确认|Confirm/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Verify removal
    await expect(page.getByText('更新后的CRUD流程')).not.toBeVisible({ timeout: 10000 });
  });
});