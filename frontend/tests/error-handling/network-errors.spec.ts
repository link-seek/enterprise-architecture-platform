// spec: specs/eap-test-plan.md
import { test, expect } from '@playwright/test';

test.describe('Error Handling - Network Errors', () => {
  test.beforeEach(async ({ page }) => {
    // Login before each test
    await page.goto('/login');
    await page.getByRole('textbox', { name: '邮箱' }).fill('test@example.com');
    await page.getByRole('textbox', { name: '密码' }).fill('testpassword123');
    await page.getByRole('button', { name: '登录' }).click();
    await expect(page).toHaveURL('/spaces/00000000-0000-0000-0000-000000000010/architectures/value-streams');
  });

  test('Data Loading States', async ({ page }) => {
    // Navigate to each main page and observe loading states
    const pages = [
      { path: '/spaces/00000000-0000-0000-0000-000000000010/architectures/value-streams', name: '价值流' },
      { path: '/spaces/00000000-0000-0000-0000-000000000010/architectures/capabilities', name: '业务能力' },
      { path: '/spaces/00000000-0000-0000-0000-000000000010/architectures/processes', name: '业务流程' },
    ];

    for (const pageInfo of pages) {
      await page.goto(pageInfo.path);
      
      // Check for loading state (if implemented)
      const loadingIndicator = page.getByText('加载中...');
      if (await loadingIndicator.isVisible()) {
        await expect(loadingIndicator).toBeVisible();
        // Wait for loading to complete
        await expect(loadingIndicator).not.toBeVisible({ timeout: 10000 });
      }
      
      // Verify data loads successfully
      await expect(page.getByRole('heading', { name: pageInfo.name })).toBeVisible();
      
      // Check for empty state if no data
      // The table might show "No data" or similar message
      const noDataMessage = page.getByText(/暂无数据|No data|Empty/);
      if (await noDataMessage.isVisible()) {
        console.log(`No data found on ${pageInfo.name} page`);
      }
      
      // Verify UI is functional (buttons, etc.)
      await expect(page.getByRole('button', { name: /新建|New/ })).toBeVisible();
    }
  });

  test('API Failure Handling - Value Streams Page', async ({ page }) => {
    // Navigate to value streams page
    await page.goto('/spaces/00000000-0000-0000-0000-000000000010/architectures/value-streams');
    
    // Simulate network failure by blocking API calls
    await page.route('**/graphql', async route => {
      // Simulate network error
      await route.abort('failed');
    });
    
    // Reload page to trigger API call with network failure
    await page.reload();
    
    // Verify error message displayed
    const errorMessage = page.getByText(/加载失败|Error|Failed to load|Network error/i);
    await expect(errorMessage).toBeVisible({ timeout: 10000 });
    
    // Verify UI doesn't crash
    await expect(page.getByRole('heading', { name: '价值流' })).toBeVisible();
    await expect(page.getByRole('button', { name: '新建价值流' })).toBeVisible();
    
    // Verify retry mechanism (if implemented)
    const retryButton = page.getByRole('button', { name: /重试|Retry/ });
    if (await retryButton.isVisible()) {
      await retryButton.click();
      // After clicking retry, we should see loading state again
      const loadingIndicator = page.getByText('加载中...');
      if (await loadingIndicator.isVisible()) {
        await expect(loadingIndicator).toBeVisible();
      }
    }
  });

  test('Empty States Handling', async ({ page }) => {
    // This test assumes we can clear data or the system has no data
    // For now, we'll just verify the UI handles empty states gracefully
    
    // Navigate to value streams page
    await page.goto('/spaces/00000000-0000-0000-0000-000000000010/architectures/value-streams');
    
    // Check for empty state messaging
    const emptyState = page.getByText(/暂无数据|No data|Empty/);
    const table = page.getByRole('table');
    
    if (await emptyState.isVisible()) {
      // Empty state is shown
      await expect(emptyState).toBeVisible();
      
      // Verify "create new" buttons still available
      await expect(page.getByRole('button', { name: '新建价值流' })).toBeVisible();
      
      // Create button should work even in empty state
      await page.getByRole('button', { name: '新建价值流' }).click();
      await expect(page.getByRole('dialog')).toBeVisible();
      await page.getByRole('button', { name: /取消|Cancel/ }).or(page.locator('button[aria-label="Close"]')).click();
    } else if (await table.isVisible()) {
      // Table has data
      console.log('Table has data, empty state not shown');
    }
  });

  test('GraphQL Query Error Handling', async ({ page }) => {
    // Navigate to value streams page
    await page.goto('/spaces/00000000-0000-0000-0000-000000000010/architectures/value-streams');
    
    // Simulate GraphQL query error by returning error response
    await page.route('**/graphql', async route => {
      // Return a GraphQL error response
      await route.fulfill({
        status: 200,
        contentType: 'application/json',
        body: JSON.stringify({
          errors: [
            {
              message: 'GraphQL query error: Cannot query field "invalidField" on type "Query"',
              locations: [{ line: 2, column: 3 }],
              path: ['valueStreams']
            }
          ],
          data: null
        })
      });
    });
    
    // Reload page to trigger query with error
    await page.reload();
    
    // Verify error boundaries catch and display errors
    const errorMessage = page.getByText(/GraphQL query error|Error loading data|Something went wrong/i);
    await expect(errorMessage).toBeVisible({ timeout: 10000 });
    
    // Verify UI doesn't crash
    await expect(page.getByRole('heading', { name: '价值流' })).toBeVisible();
    
    // Verify user can retry or navigate away
    // Check for retry button or navigation still works
    await page.getByRole('link', { name: '业务能力' }).click();
    await expect(page).toHaveURL('/spaces/00000000-0000-0000-0000-000000000010/architectures/capabilities');
  });

  test('GraphQL Mutation Error Handling', async ({ page }) => {
    // Navigate to value streams page
    await page.goto('/spaces/00000000-0000-0000-0000-000000000010/architectures/value-streams');
    
    // Simulate GraphQL mutation error
    await page.route('**/graphql', async route => {
      const request = route.request();
      const postData = request.postData();
      
      if (postData && postData.includes('mutation')) {
        // Return error for mutations
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            errors: [
              {
                message: 'Mutation failed: Validation error',
                locations: [{ line: 2, column: 3 }],
                path: ['createValueStream']
              }
            ],
            data: { createValueStream: null }
          })
        });
      } else {
        // Allow queries to proceed normally
        await route.continue();
      }
    });
    
    // Try to create a value stream (should trigger mutation error)
    await page.getByRole('button', { name: '新建价值流' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Fill form
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('测试错误处理');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('测试GraphQL错误处理');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.fill('active');
    
    // Submit form
    await page.getByRole('button', { name: /保存|Save/ }).click();
    
    // Verify error messages displayed to user
    const errorMessage = page.getByText(/Mutation failed|Validation error|保存失败|Save failed/i);
    await expect(errorMessage).toBeVisible({ timeout: 10000 });
    
    // Verify form state preserved for correction
    // Dialog should remain open with form data intact
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('textbox', { name: /名称|Name/ })).toHaveValue('测试错误处理');
    
    // Verify user can retry or cancel
    await page.getByRole('button', { name: /取消|Cancel/ }).or(page.locator('button[aria-label="Close"]')).click();
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });

  test('Session Expiry During Operation', async ({ page }) => {
    // Start a create operation
    await page.getByRole('button', { name: '新建价值流' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Fill form partially
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('会话过期测试');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('测试会话过期处理');
    
    // Simulate session expiry by clearing tokens
    await page.evaluate(() => {
      localStorage.removeItem('auth_token');
      localStorage.removeItem('refresh_token');
    });
    
    // Try to complete operation
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.fill('active');
    
    await page.getByRole('button', { name: /保存|Save/ }).click();
    
    // Verify user redirected to login
    await expect(page).toHaveURL('/login', { timeout: 10000 });
    
    // Verify clear messaging about session expiry
    await expect(page.getByText(/会话已过期|Session expired|请重新登录/i)).toBeVisible();
  });

  test('Network Interruption During Operation', async ({ page }) => {
    // Start a data-intensive operation
    await page.getByRole('button', { name: '新建价值流' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Fill form with large data
    const longDescription = '这是一个很长的描述内容，用于测试网络中断时的错误处理。'.repeat(50);
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('网络中断测试');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill(longDescription);
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');
    
    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.fill('active');
    
    // Simulate network disconnection before submission
    await page.route('**/graphql', async route => {
      await route.abort('internetdisconnected');
    });
    
    // Submit form
    await page.getByRole('button', { name: /保存|Save/ }).click();
    
    // Verify operation fails gracefully
    const errorMessage = page.getByText(/网络错误|Network error|连接失败|Failed to connect/i);
    await expect(errorMessage).toBeVisible({ timeout: 10000 });
    
    // Verify retry mechanism (if implemented)
    const retryButton = page.getByRole('button', { name: /重试|Retry/ });
    if (await retryButton.isVisible()) {
      await retryButton.click();
      // Should attempt to submit again
    }
    
    // Verify user can recover or cancel
    const cancelButton = page.getByRole('button', { name: /取消|Cancel/ });
    if (await cancelButton.isVisible()) {
      await cancelButton.click();
      await expect(page.getByRole('dialog')).not.toBeVisible();
    }
  });

  test('Browser Back/Forward Navigation During Operations', async ({ page }) => {
    // Open create dialog
    await page.getByRole('button', { name: '新建价值流' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Fill some data
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('浏览器导航测试');
    
    // Click browser back button
    await page.goBack();
    
    // Verify dialog closes gracefully
    await expect(page.getByRole('dialog')).not.toBeVisible();
    
    // Verify no data loss (dialog state should be reset)
    // Re-open dialog to check
    await page.getByRole('button', { name: '新建价值流' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    
    // Form should be empty
    const nameField = page.getByRole('textbox', { name: /名称|Name/ });
    await expect(nameField).toHaveValue('');
    
    // Close dialog
    await page.getByRole('button', { name: /取消|Cancel/ }).or(page.locator('button[aria-label="Close"]')).click();
    
    // Test with browser forward navigation
    await page.goForward();
    // Should return to value streams page
    await expect(page).toHaveURL('/spaces/00000000-0000-0000-0000-000000000010/architectures/value-streams');
    
    // Test refresh during operation
    await page.getByRole('button', { name: '新建价值流' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    
    await page.getByRole('textbox', { name: /名称|Name/ }).fill('刷新测试');
    
    // Refresh page
    await page.reload();
    
    // After refresh, dialog should be closed
    await expect(page.getByRole('dialog')).not.toBeVisible();
    // Should be on value streams page
    await expect(page).toHaveURL('/spaces/00000000-0000-0000-0000-000000000010/architectures/value-streams');
  });
});