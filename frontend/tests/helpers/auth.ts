// Shared test helpers for E2E tests
import { Page, expect } from '@playwright/test';

// Test credentials - register a fresh user if needed before test suite
export const TEST_EMAIL = 'e2e3@test.com';
export const TEST_PASSWORD = 'e2e123456';
export const TEST_NAME = 'E2E Test 3';

// Test space id mirrors backend migration TEST_SPACE_ID.
export const TEST_SPACE_ID = '00000000-0000-0000-0000-000000000010';
export const SPACE_BASE = `/spaces/${TEST_SPACE_ID}/architectures`;

/**
 * Login via the UI. Uses form submit (Enter key) which is more reliable than button click.
 * After login, verifies redirect to the space-scoped value-streams page.
 */
export async function login(page: Page) {
  await page.goto('/login');
  await page.fill('input[type="email"]', TEST_EMAIL);
  await page.fill('input[type="password"]', TEST_PASSWORD);
  await page.press('input[type="password"]', 'Enter');
  await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`, { timeout: 10000 });
}

/**
 * Logout via the UI.
 */
export async function logout(page: Page) {
  await page.getByText('退出登录').click();
  await expect(page).toHaveURL('/login', { timeout: 5000 });
}

/**
 * Ensure user is logged out before test.
 */
export async function ensureLoggedOut(page: Page) {
  await page.goto('/');
  await page.evaluate(() => localStorage.clear());
}
