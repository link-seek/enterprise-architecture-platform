// spec: issue #291 — 直接浏览空间的 E2E 测试用例
import { test, expect } from '../helpers/graphql-aware';
import { login, SPACE_BASE, TEST_SPACE_ID } from '../helpers/auth';

test.describe('Spaces - Browse Space Content', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('Browse value stream list', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/value-streams`);
    await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`);

    await expect(page.getByRole('heading', { name: '价值流' }).first()).toBeVisible({ timeout: 10000 });

    await expect(page.getByText(/加载失败|加载中/)).not.toBeVisible({ timeout: 5000 });

    const viewButtons = page.getByRole('button', { name: '查看' });
    const count = await viewButtons.count();
    if (count === 0) {
      await expect(page.getByText(/暂无|空/).first()).toBeVisible();
    }
  });

  test('Browse business capability list', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/capabilities`);
    await expect(page).toHaveURL(`${SPACE_BASE}/capabilities`);

    await expect(page.getByRole('heading', { name: '业务能力' }).first()).toBeVisible({ timeout: 10000 });

    await expect(page.getByText(/加载失败|加载中/)).not.toBeVisible({ timeout: 5000 });
  });

  test('Browse business process list', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/processes`);
    await expect(page).toHaveURL(`${SPACE_BASE}/processes`);

    await expect(page.getByRole('heading', { name: '业务流程' }).first()).toBeVisible({ timeout: 10000 });

    await expect(page.getByText(/加载失败|加载中/)).not.toBeVisible({ timeout: 5000 });
  });

  test('View value stream detail and return', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/value-streams`);
    await expect(page.getByRole('heading', { name: '价值流' }).first()).toBeVisible({ timeout: 10000 });

    await expect(page.getByText(/加载失败/)).not.toBeVisible({ timeout: 5000 });

    const viewButtons = page.getByRole('button', { name: '查看' });
    const count = await viewButtons.count();
    test.skip(count === 0, 'No value streams to view detail');

    await viewButtons.first().click();
    await expect(page).toHaveURL(/\/architectures\/value-streams\/.+/);

    await expect(page.getByText(/名称|描述|版本|状态/).first()).toBeVisible({ timeout: 5000 });

    const backButton = page.getByRole('button', { name: '返回列表' });
    if (await backButton.isVisible()) {
      await backButton.click();
    } else {
      await page.goBack();
    }
    await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`);
  });

  test('Space detail shows correct stats counts', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`/spaces/${TEST_SPACE_ID}`);
    await expect(page.getByText('加载中')).not.toBeVisible({ timeout: 10000 });

    // The three stat cards (价值流 / 业务能力 / 业务流程) each show a
    // non-negative integer count.
    const statLabels = ['价值流', '业务能力', '业务流程'];
    for (const label of statLabels) {
      const card = page.getByRole('link', { name: new RegExp(`^${label} \\d+`) }).first();
      await expect(card).toBeVisible({ timeout: 10000 });
      const numberText = card.locator('p.text-3xl');
      await expect(numberText).toBeVisible();
      const value = await numberText.textContent();
      const parsed = parseInt(value ?? '', 10);
      expect(parsed).toBeGreaterThanOrEqual(0);
      expect(Number.isNaN(parsed)).toBe(false);
    }
  });

  test('Space detail nav cards link to correct pages', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`/spaces/${TEST_SPACE_ID}`);
    await expect(page.getByText('加载中')).not.toBeVisible({ timeout: 10000 });

    // Click the 价值流 nav card and verify navigation.
    const valueStreamCard = page.getByRole('link', { name: /价值流/ }).first();
    await valueStreamCard.click();
    await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`);
  });
});
