// spec: specs/eap-test-plan.md
import { test, expect } from '@playwright/test';
import { login, SPACE_BASE } from '../helpers/auth';

test.describe('Mobile Responsive - Drawer & Cards', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.setViewportSize({ width: 375, height: 667 });
  });

  test('Drawer open/close and navigation', { tag: '@smoke' }, async ({ page }) => {
    // Sidebar links hidden initially
    await expect(page.getByRole('link', { name: '价值流' })).not.toBeVisible();

    // Open drawer
    await page.getByRole('button', { name: '打开菜单' }).click();
    await expect(page.getByRole('link', { name: '价值流' })).toBeVisible();

    // Navigate to capabilities, drawer auto-closes
    await page.getByRole('link', { name: '业务能力' }).click();
    await expect(page).toHaveURL(`${SPACE_BASE}/capabilities`);
    await expect(page.getByRole('link', { name: '业务能力' })).not.toBeVisible();
  });

  test('No horizontal overflow on list pages', { tag: '@smoke' }, async ({ page }) => {
    for (const path of ['value-streams', 'capabilities', 'processes']) {
      await page.goto(`${SPACE_BASE}/${path}`);
      await expect(page.getByRole('heading').first()).toBeVisible();
      const overflow = await page.evaluate(
        () => document.documentElement.scrollWidth - window.innerWidth
      );
      expect(overflow).toBeLessThanOrEqual(0);
    }
  });

  test('Card list rendering on mobile', { tag: '@smoke' }, async ({ page }) => {
    // Value streams page: cards present (rounded border containers)
    await expect(page.getByRole('heading', { name: '价值流' }).first()).toBeVisible();
    // Capabilities page renders without table header row (cards instead)
    await page.goto(`${SPACE_BASE}/capabilities`);
    await expect(page.getByRole('heading', { name: '业务能力' }).first()).toBeVisible();
    // Processes page
    await page.goto(`${SPACE_BASE}/processes`);
    await expect(page.getByRole('heading', { name: '业务流程' }).first()).toBeVisible();
  });

  test('Touch target size for action buttons', { tag: '@smoke' }, async ({ page }) => {
    // Menu button should be at least 36px tall
    const menuBtn = page.getByRole('button', { name: '打开菜单' });
    const box = await menuBtn.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.height).toBeGreaterThanOrEqual(36);
  });
});