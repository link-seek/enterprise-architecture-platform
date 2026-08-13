// spec: specs/eap-test-plan.md
import { test, testWithError, expect } from '../helpers/graphql-aware';
import { login, SPACE_BASE } from '../helpers/auth';

test.describe('Error Handling - Network Errors', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('Data Loading States', { tag: '@smoke' }, async ({ page }) => {
    const pages = [
      { path: `${SPACE_BASE}/value-streams`, name: '价值流' },
      { path: `${SPACE_BASE}/capabilities`, name: '业务能力' },
      { path: `${SPACE_BASE}/processes`, name: '业务流程' },
    ];

    for (const pageInfo of pages) {
      await page.goto(pageInfo.path);
      await expect(page.getByRole('heading', { name: pageInfo.name, exact: true })).toBeVisible({ timeout: 10000 });

      const noDataMessage = page.getByText(/暂无数据|No data|Empty/);
      if (await noDataMessage.isVisible()) {
        console.log(`No data found on ${pageInfo.name} page`);
      }

      // canEdit is gated by useSpaceMembership, which requires fetchMe() +
      // membership query to complete after the page reload. Use an explicit
      // timeout to avoid flaky failures in production with network latency.
      await expect(page.getByRole('button', { name: /新建|New/ })).toBeVisible({ timeout: 10000 });
    }
  });

  test('Empty States Handling', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/value-streams`);

    const emptyState = page.getByText(/暂无数据|No data|Empty/);
    const table = page.getByRole('table');

    if (await emptyState.isVisible()) {
      await expect(emptyState).toBeVisible();
      await expect(page.getByRole('button', { name: '新建价值流' })).toBeVisible();
      await page.getByRole('button', { name: '新建价值流' }).click();
      await expect(page.getByRole('dialog')).toBeVisible();
      await page.getByRole('button', { name: /取消|Cancel/ }).or(page.locator('button[aria-label="Close"]')).click();
    } else if (await table.isVisible()) {
      console.log('Table has data, empty state not shown');
    }
  });
});

// Error-handling tests that intentionally trigger GraphQL/network errors.
// These use testWithError which suppresses the automatic GraphQL error
// detection (since the errors are expected by design).
testWithError.describe('Error Handling - Network Errors (Intentional Errors)', () => {
  testWithError.beforeEach(async ({ page }) => {
    await login(page);
  });

  testWithError('API Failure Handling - Value Streams Page', { tag: '@regression' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/value-streams`);

    // Block only the value-streams query; allow membership queries so the
    // "新建价值流" button (gated by canEdit) still renders.
    await page.route('**/graphql', async route => {
      const postData = route.request().postData();
      if (postData && postData.includes('valueStreamsBySpace')) {
        await route.abort('failed');
      } else {
        await route.continue();
      }
    });

    await page.reload();

    await expect(page.getByText(/加载失败/i)).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole('heading', { name: '价值流', exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: '新建价值流' })).toBeVisible();
  });

  testWithError('GraphQL Query Error Handling', { tag: '@regression' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/value-streams`);

    // Return a GraphQL error only for value-streams queries; allow
    // membership queries so the create button remains visible.
    await page.route('**/graphql', async route => {
      const postData = route.request().postData();
      if (postData && postData.includes('valueStreamsBySpace')) {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            errors: [{ message: 'GraphQL query error: Cannot query field "invalidField" on type "Query"' }],
            data: null,
          }),
        });
      } else {
        await route.continue();
      }
    });

    await page.reload();

    await expect(page.getByText(/加载失败/i)).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole('heading', { name: '价值流', exact: true })).toBeVisible();

    // Navigation still works (membership query is not blocked)
    await page.getByRole('link', { name: '业务能力', exact: true }).click();
    await expect(page).toHaveURL(`${SPACE_BASE}/capabilities`);
  });

  testWithError('GraphQL Mutation Error Handling', { tag: '@regression' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/value-streams`);

    // Simulate GraphQL mutation error — only block mutations, allow queries
    await page.route('**/graphql', async route => {
      const postData = route.request().postData();
      if (postData && postData.includes('mutation')) {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            errors: [{ message: 'Mutation failed: Validation error' }],
            data: { createValueStream: null },
          }),
        });
      } else {
        await route.continue();
      }
    });

    await page.getByRole('button', { name: '新建价值流' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();

    await page.getByRole('textbox', { name: /名称|Name/ }).fill('测试错误处理');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('测试GraphQL错误处理');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');

    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.selectOption('active');

    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();

    await expect(page.getByText(/Mutation failed|Validation error/i)).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('textbox', { name: /名称|Name/ })).toHaveValue('测试错误处理');

    await page.getByRole('button', { name: /取消|Cancel/ }).or(page.locator('button[aria-label="Close"]')).click();
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });

  testWithError('Session Expiry During Operation', { tag: '@regression' }, async ({ page }) => {
    // Login lands on the architecture overview; navigate to value-streams explicitly.
    await page.goto(`${SPACE_BASE}/value-streams`);
    await page.getByRole('button', { name: '新建价值流' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();

    await page.getByRole('textbox', { name: /名称|Name/ }).fill('会话过期测试');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('测试会话过期处理');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');

    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.selectOption('active');

    // Simulate session expiry by clearing the auth token used by Apollo
    await page.evaluate(() => {
      localStorage.removeItem('access_token');
      localStorage.removeItem('refresh_token');
    });

    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();

    // The dialog shows the error inline (no automatic redirect to /login
    // because ProtectedRoute only checks on initial render)
    await expect(page.getByText(/Authentication required|操作失败|Unauthorized|Forbidden|未授权|权限/i)).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole('dialog')).toBeVisible();
  });

  testWithError('Network Interruption During Operation', { tag: '@regression' }, async ({ page }) => {
    // Login lands on the architecture overview; navigate to value-streams explicitly.
    await page.goto(`${SPACE_BASE}/value-streams`);
    await page.getByRole('button', { name: '新建价值流' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();

    await page.getByRole('textbox', { name: /名称|Name/ }).fill('网络中断测试');
    await page.getByRole('textbox', { name: /描述|Description/ }).fill('测试网络中断时的错误处理');
    await page.getByRole('textbox', { name: /版本|Version/ }).fill('v1.0');

    const statusField = page.getByRole('combobox', { name: /状态|Status/ }).or(page.getByRole('textbox', { name: /状态|Status/ }));
    await statusField.selectOption('active');

    // Simulate network disconnection before submission — only block mutations
    await page.route('**/graphql', async route => {
      const postData = route.request().postData();
      if (postData && postData.includes('mutation')) {
        await route.abort('failed');
      } else {
        await route.continue();
      }
    });

    await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();

    // The dialog remains open with the error
    await expect(page.getByRole('dialog')).toBeVisible({ timeout: 10000 });

    // User can cancel to recover
    const cancelButton = page.getByRole('button', { name: /取消|Cancel/ });
    await expect(cancelButton).toBeVisible({ timeout: 10000 });
    await cancelButton.click();
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });

  testWithError('Browser Back/Forward Navigation During Operations', { tag: '@regression' }, async ({ page }) => {
    // Login lands on the architecture overview; navigate to value-streams explicitly.
    await page.goto(`${SPACE_BASE}/value-streams`);
    await page.getByRole('button', { name: '新建价值流' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();

    await page.getByRole('textbox', { name: /名称|Name/ }).fill('浏览器导航测试');

    // Click browser back button — dialog closes and page navigates
    await page.goBack();

    // Navigate back to value streams page explicitly
    await page.goto(`${SPACE_BASE}/value-streams`);
    await expect(page.getByRole('heading', { name: '价值流', exact: true })).toBeVisible();

    // Re-open dialog — form should be empty (state reset)
    await page.getByRole('button', { name: '新建价值流' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();

    const nameField = page.getByRole('textbox', { name: /名称|Name/ });
    await expect(nameField).toHaveValue('');

    // Close dialog
    await page.getByRole('button', { name: /取消|Cancel/ }).or(page.locator('button[aria-label="Close"]')).click();

    // Test refresh during operation
    await page.getByRole('button', { name: '新建价值流' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();

    await page.getByRole('textbox', { name: /名称|Name/ }).fill('刷新测试');

    await page.reload();

    // After refresh, dialog should be closed
    await expect(page.getByRole('dialog')).not.toBeVisible();
    await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`);
  });
});