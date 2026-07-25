// spec: specs/eap-test-plan.md
import { test, expect } from '@playwright/test';
import { login, logout, ensureLoggedOut, TEST_EMAIL, TEST_PASSWORD } from '../helpers/auth';

test.describe('Authentication - Login', () => {
  test.beforeEach(async ({ page }) => {
    await ensureLoggedOut(page);
  });

  test('Happy Path - User Login', async ({ page }) => {
    await page.goto('/login');
    
    await expect(page.getByText('企业架构平台')).toBeVisible();
    await expect(page.getByText('登录以继续')).toBeVisible();
    
    await page.fill('input[type="email"]', TEST_EMAIL);
    await page.fill('input[type="password"]', TEST_PASSWORD);
    await page.press('input[type="password"]', 'Enter');
    
    await expect(page).toHaveURL('/spaces/00000000-0000-0000-0000-000000000010/architectures/value-streams', { timeout: 10000 });
    await expect(page.getByText('E2E Test 3')).toBeVisible({ timeout: 5000 });
  });

  test('Edge Case - Invalid Login Credentials', async ({ page }) => {
    await page.goto('/login');
    
    await page.fill('input[type="email"]', 'wrong@example.com');
    await page.fill('input[type="password"]', 'wrongpassword');
    await page.press('input[type="password"]', 'Enter');
    
    await expect(page.getByText(/invalid credentials/i)).toBeVisible({ timeout: 5000 });
    await expect(page).toHaveURL('/login');
  });

  test('Happy Path - User Logout', async ({ page }) => {
    await login(page);
    await logout(page);
  });

  test('Edge Case - Protected Route Access', async ({ page }) => {
    await page.goto('/spaces/00000000-0000-0000-0000-000000000010/architectures/value-streams');
    await expect(page).toHaveURL('/login');
    await expect(page.getByText('企业架构平台')).toBeVisible();
  });
});
