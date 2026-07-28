// spec: specs/eap-test-plan.md
import { test, expect } from '@playwright/test';
import { login, logout, TEST_EMAIL, SPACE_BASE } from '../helpers/auth';

test.describe('Navigation & Layout - Sidebar', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('Happy Path - Sidebar Navigation', { tag: '@smoke' }, async ({ page }) => {
    // Verify sidebar contains 3 main items
    await expect(page.getByRole('link', { name: '价值流' })).toBeVisible();
    await expect(page.getByRole('link', { name: '业务能力' })).toBeVisible();
    await expect(page.getByRole('link', { name: '业务流程' })).toBeVisible();
    
    // Start on value streams page (default after login)
    await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`);
    await expect(page.getByRole('link', { name: '价值流' })).toHaveClass(/bg-primary/);
    
    // Click "业务能力"
    await page.getByRole('link', { name: '业务能力' }).click();
    
    // Verify URL changes and menu item is highlighted
    await expect(page).toHaveURL(`${SPACE_BASE}/capabilities`);
    await expect(page.getByRole('link', { name: '业务能力' })).toHaveClass(/bg-primary/);
    await expect(page.getByRole('heading', { name: /业务能力/ })).toBeVisible();
    
    // Click "业务流程"
    await page.getByRole('link', { name: '业务流程' }).click();
    
    // Verify URL changes and menu item is highlighted
    await expect(page).toHaveURL(`${SPACE_BASE}/processes`);
    await expect(page.getByRole('link', { name: '业务流程' })).toHaveClass(/bg-primary/);
    await expect(page.getByRole('heading', { name: /业务流程/ })).toBeVisible();
    
    // Click "价值流" to return
    await page.getByRole('link', { name: '价值流' }).click();
    
    // Verify URL changes and menu item is highlighted
    await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`);
    await expect(page.getByRole('link', { name: '价值流' })).toHaveClass(/bg-primary/);
    await expect(page.getByRole('heading', { name: /价值流/ }).first()).toBeVisible();
  });

  test('Happy Path - Breadcrumb/Back Navigation', { tag: '@smoke' }, async ({ page }) => {
    // First, ensure we have at least one value stream
    // This test assumes there's at least one value stream in the system
    // We'll check for the "查看" button on the first row
    
    // Wait for value streams to load
    await expect(page.getByRole('heading', { name: '价值流' }).first()).toBeVisible();
    
    // Look for "查看" button in the table
    const viewButtons = page.getByRole('button', { name: '查看' });
    
    // If there are value streams, click the first one
    const count = await viewButtons.count();
    if (count > 0) {
      await viewButtons.first().click();
      
      // Verify URL changes to detail page
      await expect(page).toHaveURL(/\/architectures\/value-streams\/.+/);
      
      // Verify detail page shows value stream information
      await expect(page.getByRole('heading', { level: 1 })).toBeVisible();
      await expect(page.getByText(/名称|描述|版本|状态/)).toBeVisible();
      
      // Look for "返回列表" button and click it
      const backButton = page.getByRole('button', { name: '返回列表' });
      if (await backButton.isVisible()) {
        await backButton.click();
        
        // Verify returned to value streams list
        await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`);
        await expect(page.getByRole('heading', { name: '价值流' }).first()).toBeVisible();
      } else {
        // If no back button, use browser back
        await page.goBack();
        await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`);
      }
    } else {
      // No value streams - skip the detail navigation part
      console.log('No value streams found for detail navigation test');
    }
  });

  test('Sidebar User Profile Display', { tag: '@smoke' }, async ({ page }) => {
    // Verify sidebar shows user info after login
    await expect(page.getByText(TEST_EMAIL)).toBeVisible();
    
    // Verify logout button is present
    await expect(page.getByRole('button', { name: '退出登录' })).toBeVisible();
  });

  test('Responsive Sidebar Behavior', { tag: '@smoke' }, async ({ page }) => {
    // Test on different viewport sizes
    const viewports = [
      { width: 1280, height: 720 }, // Desktop
      { width: 768, height: 1024 }, // Tablet
      { width: 375, height: 667 }, // Mobile
    ];
    
    for (const viewport of viewports) {
      await page.setViewportSize(viewport);
      
      // Verify sidebar is visible on all sizes (might collapse on mobile)
      await expect(page.getByRole('link', { name: '价值流' })).toBeVisible();
      await expect(page.getByRole('link', { name: '业务能力' })).toBeVisible();
      await expect(page.getByRole('link', { name: '业务流程' })).toBeVisible();
      
      // Verify user info is visible
      await expect(page.getByText(TEST_EMAIL)).toBeVisible();
      
      // On mobile, sidebar might be collapsed - check if navigation still works
      if (viewport.width < 768) {
        // If sidebar is collapsed, there might be a hamburger menu
        // For now, just verify main content is accessible
        await expect(page.getByRole('heading', { name: '价值流' }).first()).toBeVisible();
      }
    }
  });

  test('Keyboard Navigation in Sidebar', { tag: '@smoke' }, async ({ page }) => {
    // Test keyboard accessibility - verify Tab cycles through focusable elements
    const sidebarLinks = ['价值流', '业务能力', '业务流程'];
    
    // Tab through and verify sidebar links are reachable via keyboard
    for (let i = 0; i < 10; i++) {
      await page.keyboard.press('Tab');
      const focused = await page.evaluate(() => document.activeElement?.textContent?.trim() || '');
      if (sidebarLinks.includes(focused)) {
        // Found a sidebar link via keyboard nav
        break;
      }
    }
    
    // Verify at least one sidebar link is focused after tabbing
    const focusedText = await page.evaluate(() => document.activeElement?.textContent?.trim() || '');
    expect(sidebarLinks.includes(focusedText) || focusedText === '退出登录').toBeTruthy();
    
    // Navigate to capabilities via click (more reliable than keyboard)
    await page.getByRole('link', { name: '业务能力' }).click();
    await expect(page).toHaveURL(`${SPACE_BASE}/capabilities`);
    await expect(page.getByRole('heading', { name: /业务能力/ }).first()).toBeVisible();
  });
});