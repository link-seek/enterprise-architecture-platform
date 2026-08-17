// spec: specs/eap-test-plan.md
import { test, testWithError, expect } from '../helpers/graphql-aware';
import { login, SPACE_BASE } from '../helpers/auth';

test.describe('Value Stream Management - CRUD Operations', () => {
  test.beforeEach(async ({ page }) => {
    // Login before each test (env-driven credentials for multi-environment reuse).
    // Login now lands on the architecture overview; navigate to value-streams explicitly.
    await login(page);
    await page.goto(`${SPACE_BASE}/value-streams`);
  });

  test('Happy Path - Create Value Stream', { tag: ['@smoke', '@regression'] }, async ({ page }) => {
    // Click "新建价值流" button
    await page.getByRole('button', { name: '新建价值流' }).click();
    
    // Verify create dialog opens
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: /新建价值流|创建价值流/ })).toBeVisible();
    
    // Fill in form
    const name = `测试价值流_${Date.now()}`;
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(name);
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('这是一个测试价值流');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    // Select status (assuming it's a select/dropdown)
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.selectOption('active');
    
    // Select importance (assuming it's a select/dropdown)
    const importanceField = page.getByRole('combobox', { name: /重要性|Importance/ }).or(page.getByRole('textbox', { name: /重要性|Importance/ }));
    await importanceField.selectOption('High');
    
    // Click "保存" button
    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    
    // Verify dialog closes
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Verify new value stream appears in table
    const row = page.locator('tr').filter({ hasText: name });
    await expect(row).toBeVisible({ timeout: 10000 });
    
    // Verify table shows correct data
    await expect(row.getByText('v1.0')).toBeVisible();
    await expect(row.getByText('active')).toBeVisible();
    
    // Note: Pagination count verification would require checking the table structure
    // For now, we verify the item appears in the table
  });

  test('Happy Path - Edit Value Stream', { tag: '@regression' }, async ({ page }) => {
    // First, create a value stream to edit
    await page.getByRole('button', { name: '新建价值流' }).click();
    const originalName = `原始名称_${Date.now()}`;
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(originalName);
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('原始描述');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.selectOption('active');
    
    const importanceField = page.getByRole('combobox', { name: /重要性|Importance/ }).or(page.getByRole('textbox', { name: /重要性|Importance/ }));
    await importanceField.selectOption('Medium');
    
    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the created value stream and click edit button
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
    await expect(descField).toHaveValue('原始描述');
    
    // Modify fields
    const updatedName = `Updated_${Date.now()}`;
    await nameField.fill(updatedName);
    await descField.fill('Updated Description');
    
    // Click "保存" button
    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    
    // Verify dialog closes
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Verify table shows updated data
    const updatedRow = page.locator('tr').filter({ hasText: updatedName });
    await expect(updatedRow).toBeVisible({ timeout: 10000 });
    await expect(updatedRow.getByText('Updated Description')).toBeVisible();
    
    // Verify other fields unchanged
    await expect(updatedRow.getByText('v1.0')).toBeVisible();
    await expect(updatedRow.getByText('active')).toBeVisible();
  });

  test('Happy Path - Delete Value Stream', { tag: ['@smoke', '@regression'] }, async ({ page }) => {
    // Use a unique name to avoid strict-mode violations from residual data on repeated runs
    const name = `待删除价值流_${Date.now()}`;
    // First, create a value stream to delete
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(name);
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('这个将被删除');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');

    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.selectOption('active');

    const importanceField = page.getByRole('combobox', { name: /重要性|Importance/ }).or(page.getByRole('textbox', { name: /重要性|Importance/ }));
    await importanceField.selectOption('Low');

    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });

    // Find the created value stream
    const row = page.locator('tr').filter({ hasText: name });
    await expect(row).toBeVisible();

    // Click delete (trash) button
    await row.getByRole('button').filter({ has: page.locator('svg[class*="lucide-trash-2"]') }).click();

    // Verify delete confirmation dialog opens with the delete wording
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog.getByText(/确认删除/)).toBeVisible();
    await expect(dialog.getByText(/不可恢复/)).toBeVisible();

    // Click "删除" button
    await dialog.getByRole('button', { name: '删除' }).click();

    // Verify dialog closes
    await expect(dialog).not.toBeVisible({ timeout: 10000 });

    // Soft delete removes the row from the list
    await expect(row).not.toBeVisible({ timeout: 10000 });

    // Reload and re-assert to catch optimistic-update "fake deletes" (#403).
    await page.reload();
    await expect(page.getByText(name, { exact: true })).not.toBeVisible({ timeout: 10000 });
  });

  test('Edge Case - Create Value Stream Validation', { tag: '@regression' }, async ({ page }) => {
    await page.getByRole('button', { name: '新建价值流' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Test Case 1: Empty name field — submit button should be disabled
    await page.getByRole('textbox', { name: /名称|Name/ }).clear();
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('描述');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    const submitButton = page.getByRole('button', { name: /保存|创建|Save|Create/ });
    await expect(submitButton).toBeDisabled();
    
    // Dialog should stay open
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Test Case 2: Fill name — button should become enabled
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('测试名称');
    await expect(submitButton).toBeEnabled();
    
    // Close dialog
    await page.getByRole('button', { name: /取消|Cancel/ }).or(page.locator('button[aria-label="Close"]')).click();
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });

  test('Edge Case - Delete Confirmation Cancel', { tag: '@regression' }, async ({ page }) => {
    // Create a value stream
    await page.getByRole('button', { name: '新建价值流' }).click();
    const cancelName = `测试取消删除_${Date.now()}`;
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(cancelName);
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('测试取消删除描述');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.selectOption('active');
    
    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the created value stream
    const row = page.locator('tr').filter({ hasText: cancelName });
    await expect(row).toBeVisible();
    
    // Click delete (trash) button
    await row.getByRole('button').filter({ has: page.locator('svg[class*="lucide-trash-2"]') }).click();
    
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
    await expect(page.getByText(cancelName)).toBeVisible();
  });

  test('View Value Stream Details', { tag: '@regression' }, async ({ page }) => {
    // Use a unique name to avoid strict-mode violations from residual data on repeated runs
    const name = `查看详情测试_${Date.now()}`;
    // First, create a value stream to view
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(name);
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('这是一个用于查看详情的测试价值流');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.selectOption('active');
    
    const importanceField = page.getByRole('combobox', { name: /重要性|Importance/ }).or(page.getByRole('textbox', { name: /重要性|Importance/ }));
    await importanceField.selectOption('High');
    
    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
    
    // Find the created value stream and click "查看" button
    const row = page.locator('tr').filter({ hasText: name });
    await expect(row).toBeVisible();
    
    // Click "查看" button
    await row.getByRole('button', { name: '查看' }).click();
    
    // Verify detail page loads
    await expect(page).toHaveURL(/\/architectures\/value-streams\/.+/);
    
    // Verify all value stream data displayed
    await expect(page.getByText(name)).toBeVisible();
    await expect(page.getByText('这是一个用于查看详情的测试价值流')).toBeVisible();
    await expect(page.getByText('v1.0')).toBeVisible();
    await expect(page.getByText('active')).toBeVisible();
    
    // Look for "返回列表" button and click it
    const backButton = page.getByRole('button', { name: '返回列表' });
    if (await backButton.isVisible()) {
      await backButton.click();
      await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`);
    } else {
      // Use browser back if no back button
      await page.goBack();
      await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`);
    }
  });
});

// Error-handling tests that intentionally trigger GraphQL errors. These use
// testWithError which suppresses the automatic GraphQL error detection.
testWithError.describe('Value Stream Delete - Error Handling', () => {
  testWithError.beforeEach(async ({ page }) => {
    await login(page);
    await page.goto(`${SPACE_BASE}/value-streams`);
  });

  testWithError('非 owner 删除价值流应显示错误提示', { tag: '@regression' }, async ({ page }) => {
    // The list gates the delete button on client-side ownership (isEntityOwner),
    // so a real non-owner cannot see it in the UI. To verify the delete dialog
    // surfaces backend permission errors instead of silently swallowing them,
    // intercept the valueStreamDelete mutation and return the FORBIDDEN error
    // the backend returns for NotEntityOwner — the exact failure mode users hit
    // when their client-side ownership view is stale relative to the backend.
    await page.route('**/graphql', async (route) => {
      const postData = route.request().postData();
      if (postData && postData.includes('valueStreamDelete')) {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            data: null,
            errors: [{ message: 'Not entity owner', extensions: { code: 'FORBIDDEN' } }],
          }),
        });
      } else {
        await route.continue();
      }
    });

    // Create a value stream as the current (owner) user so the delete button
    // is visible, then attempt to delete it — the mocked FORBIDDEN response
    // simulates a non-owner delete. The name is deliberately free of words
    // the error assertion matches on (无法删除) so the assertion can only
    // pass via the rendered error div, not the confirmation message.
    const name = `非属主删除测试_${Date.now()}`;
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称|Name/ }).fill(name);
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('用于测试删除错误提示');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');

    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.selectOption('active');

    const importanceField = page.getByRole('combobox', { name: /重要性|Importance/ }).or(page.getByRole('textbox', { name: /重要性|Importance/ }));
    await importanceField.selectOption('Low');

    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });

    // Find the created value stream and open the delete confirmation dialog.
    const row = page.locator('tr').filter({ hasText: name });
    await expect(row).toBeVisible();
    await row.getByRole('button').filter({ has: page.locator('svg[class*="lucide-trash-2"]') }).click();

    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await expect(dialog.getByText(/确认删除/)).toBeVisible();

    // Click "删除" — the mocked FORBIDDEN should surface as a friendly error
    // message and the dialog must stay open (not silently close). The text
    // "无法删除" only appears in the error div, never in the confirmation
    // message, so this can only pass once the fix renders the error.
    await dialog.getByRole('button', { name: '删除' }).click();

    await expect(dialog.getByText(/无法删除/i)).toBeVisible({ timeout: 5000 });
    await expect(dialog).toBeVisible();
  });
});