// spec: specs/eap-test-plan.md
import { test, expect } from '../helpers/graphql-aware';
import { login, TEST_SPACE_ID } from '../helpers/auth';

test.describe('Spaces - Browse Spaces Main Flow', () => {
  test.beforeEach(async ({ page }) => {
    // Login before each test. Env-driven credentials for multi-environment reuse.
    await login(page);
  });

  test('Happy Path - Browse spaces list and enter a space', { tag: '@smoke' }, async ({ page }) => {
    // Track any 405 responses on /graphql to assert the list loads without 405.
    const statuses: number[] = [];
    page.on('response', (res) => {
      if (res.url().includes('/graphql')) statuses.push(res.status());
    });

    // Navigate to spaces list via the sidebar "所有空间" link.
    await page.getByRole('link', { name: /所有空间/ }).click();
    await expect(page).toHaveURL('/spaces');

    // Spaces list page header is visible.
    await expect(page.getByRole('heading', { name: '所有空间' })).toBeVisible();

    // The list loads without error (no 405 / no failure banner).
    await expect(page.getByText(/加载失败/)).not.toBeVisible();

    // Wait for the seeded test space card to appear. List loaded successfully.
    const testSpaceCard = page.getByRole('link', { name: /测试空间/ });
    await expect(testSpaceCard).toBeVisible({ timeout: 10000 });

    // Assert no 405 was returned while loading the list.
    expect(statuses.filter((s) => s === 405)).toHaveLength(0);

    // Click into the test space → detail page loads.
    await testSpaceCard.click();
    await expect(page).toHaveURL(`/spaces/${TEST_SPACE_ID}`);

    // Detail page header shows the space name (rendered as a <span>).
    await expect(page.getByText('测试空间', { exact: true })).toBeVisible();

    // Verify the 价值流 / 业务能力 / 业务流程 navigation cards (tabs) are visible.
    await expect(page.getByRole('heading', { name: '价值流' })).toBeVisible();
    await expect(page.getByRole('heading', { name: '业务能力' })).toBeVisible();
    await expect(page.getByRole('heading', { name: '业务流程' })).toBeVisible();
  });
});