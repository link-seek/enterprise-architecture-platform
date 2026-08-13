// spec: specs/eap-test-plan.md
import { test, expect } from '@playwright/test';
import { login, logout, ensureLoggedOut, TEST_EMAIL, TEST_PASSWORD, SPACE_BASE } from '../helpers/auth';

test.describe('Authentication - Login', () => {
  test.beforeEach(async ({ page }) => {
    await ensureLoggedOut(page);
  });

  test('Happy Path - User Login', { tag: '@smoke' }, async ({ page }) => {
    await page.goto('/login');
    
    await expect(page.getByText('企业架构平台')).toBeVisible();
    await expect(page.getByText('登录以继续')).toBeVisible();
    
    await page.fill('input[type="email"]', TEST_EMAIL);
    await page.fill('input[type="password"]', TEST_PASSWORD);
    await page.press('input[type="password"]', 'Enter');
    
    // Login success: sidebar nav items + logout button visible
    await expect(page.getByRole('link', { name: '价值流' })).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole('link', { name: '业务能力' })).toBeVisible();
    await expect(page.getByRole('link', { name: '业务流程' })).toBeVisible();
    await expect(page.getByRole('button', { name: '退出登录' })).toBeVisible();
  });

  test('Edge Case - Invalid Login Credentials', { tag: '@smoke' }, async ({ page }) => {
    await page.goto('/login');
    
    await page.fill('input[type="email"]', 'wrong@example.com');
    await page.fill('input[type="password"]', 'wrongpassword');
    await page.press('input[type="password"]', 'Enter');
    
    await expect(page.getByText(/invalid credentials/i)).toBeVisible({ timeout: 5000 });
    await expect(page).toHaveURL('/login');
  });

  test('Happy Path - User Logout', { tag: '@smoke' }, async ({ page }) => {
    await login(page);
    await logout(page);
  });

  test('Edge Case - Protected Route Access', { tag: '@smoke' }, async ({ page }) => {
    // Architecture pages are public read; the admin-only /users route still
    // redirects anonymous visitors to the login page.
    await page.goto(`${SPACE_BASE}/users`);
    await expect(page).toHaveURL('/login');
    await expect(page.getByText('企业架构平台')).toBeVisible();
  });
});
