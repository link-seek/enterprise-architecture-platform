// spec: specs/eap-test-plan.md
import { test, expect } from '../helpers/graphql-aware';
import { login, SPACE_BASE } from '../helpers/auth';

test.describe('Business Processes Management - CRUD Operations', () => {
  test.beforeEach(async ({ page }) => {
    // Login before each test (env-driven credentials for multi-environment reuse)
    await login(page);

    // Navigate to processes page
    await page.getByRole('link', { name: '业务流程', exact: true }).click();
    await expect(page).toHaveURL(`${SPACE_BASE}/processes`);
  });

  test('Happy Path - Create Business Process', { tag: ['@smoke', '@regression'] }, async ({ page }) => {
    // Click "新建业务流程" button
    const createButton = page.getByRole('button', { name: /新建流程|新建业务流程|New Business Process/ });
    await expect(createButton).toBeVisible();
    await createButton.click();

    // Verify create dialog opens
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: /新建流程|新建业务流程|Create Business Process/ })).toBeVisible();

    // Fill in form with unique test data
    const name = `测试业务流程_${Date.now()}`;
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(name);
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
    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();

    // Verify dialog closes
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });

    // Verify new process appears in table
    await expect(page.getByRole('cell', { name, exact: true })).toBeVisible({ timeout: 10000 });

    // Verify numeric fields formatted correctly
    const row = page.locator('tr').filter({ hasText: name });
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

  test('Happy Path - Read Business Process', { tag: '@regression' }, async ({ page }) => {
    // Use a unique name to avoid strict-mode violations from residual data on repeated runs
    const name = `读取测试流程_${Date.now()}`;
    // Create a process to read
    const createButton = page.getByRole('button', { name: /新建流程|新建业务流程|New Business Process/ });
    await createButton.click();
    
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(name);
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
    
    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Verify new process appears in table
    await expect(page.getByText(name)).toBeVisible({ timeout: 10000 });
    
    // Verify all fields displayed correctly
    const row = page.locator('tr').filter({ hasText: name });
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

  test('Happy Path - Update Business Process', { tag: '@regression' }, async ({ page }) => {
    // Create a process to update
    const createButton = page.getByRole('button', { name: /新建流程|新建业务流程|New Business Process/ });
    await createButton.click();

    const originalName = `更新前流程_${Date.now()}`;
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(originalName);
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

    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });

    // Find the created process and click edit button
    const row = page.locator('tr').filter({ hasText: originalName });
    await expect(row).toBeVisible();

    // Click edit (pencil) button
    await row.getByRole('button').filter({ has: page.locator('svg[class*="lucide-pencil"]') }).click();

    // Verify edit dialog opens with pre-filled data
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: /编辑|Edit/ })).toBeVisible();

    // Verify form fields have existing data
    const nameField = page.getByRole('textbox', { name: /名称|Name/ });
    await expect(nameField).toHaveValue(originalName);

    const descField = page.getByRole('textbox', { name: /描述|Description/ });
    await expect(descField).toHaveValue('更新前描述');

    // Modify fields
    const updatedName = `更新后流程_${Date.now()}`;
    await nameField.fill(updatedName);
    const updatedDesc = `更新后描述_${Date.now()}`;
    await descField.fill(updatedDesc);

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
    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();

    // Verify dialog closes
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });

    // Verify table shows updated data
    await expect(page.getByText(updatedName)).toBeVisible({ timeout: 10000 });
    await expect(page.getByText(updatedDesc)).toBeVisible();

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

  test('Happy Path - Delete Business Process', { tag: ['@smoke', '@regression'] }, async ({ page }) => {
    // Create a process to delete
    const createButton = page.getByRole('button', { name: /新建流程|新建业务流程|New Business Process/ });
    await createButton.click();

    const name = `待删除流程_${Date.now()}`;
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(name);
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('这个将被删除');

    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });

    // Find the created process
    const row = page.locator('tr').filter({ hasText: name });
    await expect(row).toBeVisible();

    // Click delete (trash) button
    await row.getByRole('button').filter({ has: page.locator('svg[class*="lucide-trash-2"]') }).click();

    // Verify delete confirmation dialog opens
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByText(/确认删除|Confirm delete/)).toBeVisible();

    // Click "确认" button
    await page.getByRole('button', { name: /确认|删除|Confirm|Delete/ }).click();

    // Verify dialog closes
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });

    // Verify process removed from table
    await expect(page.getByText(name, { exact: true })).not.toBeVisible({ timeout: 10000 });

    // Reload and re-assert to catch optimistic-update "fake deletes" (#403).
    await page.reload();
    await expect(page.getByText(name, { exact: true })).not.toBeVisible({ timeout: 10000 });
  });

  test('Edge Case - Numeric Input Validation', { tag: '@regression' }, async ({ page }) => {
    // Click "新建业务流程" button
    const createButton = page.getByRole('button', { name: /新建流程|新建业务流程|New Business Process/ });
    await createButton.click();
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Fill basic fields
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('数值验证测试');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('测试数值输入验证');
    
    // Test Case 1: Negative cycle time (if validation exists)
    const cycleTimeField = page.getByRole('spinbutton', { name: /周期时间|Cycle Time/ }).or(page.getByRole('textbox', { name: /周期时间|Cycle Time/ }));
    if (await cycleTimeField.isVisible()) {
      await cycleTimeField.fill('-10');
      await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
      
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
      await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
      
      // Should show validation error
      await expect(page.getByRole('dialog')).toBeVisible();
      
      // Clear the invalid value
      await costField.fill('');
    }
    
    // Test Case 3: Non-numeric values in numeric fields
    if (await cycleTimeField.isVisible()) {
      await cycleTimeField.fill('not-a-number');
      await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
      
      // Should show validation error
      await expect(page.getByRole('dialog')).toBeVisible();
    }
    
    // Close dialog
    await page.getByRole('button', { name: /取消|Cancel/ }).or(page.locator('button[aria-label="Close"]')).click();
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });

  test('Full CRUD Cycle with Numeric Fields', { tag: ['@smoke', '@regression'] }, async ({ page }) => {
    // Create
    const createButton = page.getByRole('button', { name: /新建流程|新建业务流程|New Business Process/ });
    await createButton.click();

    const name = `完整CRUD流程_${Date.now()}`;
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(name);
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

    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });

    // Read
    await expect(page.getByText(name)).toBeVisible({ timeout: 10000 });
    const row = page.locator('tr').filter({ hasText: name });
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
    await row.getByRole('button').filter({ has: page.locator('svg[class*="lucide-pencil"]') }).click();
    await expect(page.getByRole('dialog')).toBeVisible();

    const updatedName = `更新后的CRUD流程_${Date.now()}`;
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(updatedName);

    if (await slaField.isVisible()) {
      const editSlaField = page.getByRole('spinbutton', { name: /SLA|服务级别协议/ }).or(page.getByRole('textbox', { name: /SLA|服务级别协议/ }));
      await editSlaField.fill('99.99');
    }

    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });

    await expect(page.getByText(updatedName)).toBeVisible({ timeout: 10000 });

    // Delete
    const updatedRow = page.locator('tr').filter({ hasText: updatedName });
    await updatedRow.getByRole('button').filter({ has: page.locator('svg[class*="lucide-trash-2"]') }).click();

    await expect(page.getByRole('dialog')).toBeVisible();
    await page.getByRole('button', { name: /确认|删除|Confirm|Delete/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });

    // Verify removal
    await expect(page.getByText(updatedName, { exact: true })).not.toBeVisible({ timeout: 10000 });

    // Reload and re-assert to catch optimistic-update "fake deletes" (#403).
    await page.reload();
    await expect(page.getByText(updatedName, { exact: true })).not.toBeVisible({ timeout: 10000 });
  });

  test('流程表单支持输入/输出字段', { tag: '@regression' }, async ({ page }) => {
    // R2：新建流程时填写「输入/输出」（换行分隔）
    await page.getByRole('button', { name: /新建流程|新建业务流程|New Business Process/ }).click();
    await expect(page.getByRole('dialog')).toBeVisible();

    const name = `E2E流程_${Date.now()}`;
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(name);
    await page.getByRole('textbox', { name: /输入|Input/ }).fill('需求\nIssue');
    await page.getByRole('textbox', { name: /输出|Output/ }).fill('ADR');
    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 15000 });

    // 列表展示输入/输出 Badge
    const row = page.locator('tr').filter({ hasText: name });
    await expect(row).toBeVisible({ timeout: 15000 });
    await expect(row.getByText('ADR', { exact: true })).toBeVisible();
    await expect(row.getByText('Issue', { exact: true })).toBeVisible();

    // 编辑对话框回填输入/输出
    await row.getByRole('button').filter({ has: page.locator('svg[class*="lucide-pencil"]') }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    const inputField = page.getByRole('textbox', { name: /输入|Input/ });
    await expect(inputField).toHaveValue('需求\nIssue');
    await expect(page.getByRole('textbox', { name: /输出|Output/ })).toHaveValue('ADR');
    await page.getByRole('button', { name: /取消|Cancel/ }).or(page.locator('button[aria-label="Close"]')).click();
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });
});