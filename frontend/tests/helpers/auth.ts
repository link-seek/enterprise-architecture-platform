// Shared test helpers for E2E tests
import { Page, expect } from '@playwright/test';

// Single-source credentials (E2E isolation): TEST_* reads only E2E_TEST_*,
// role/admin accounts read only APP_SEED_*; defaults match docker-compose.ci.yml.
export const TEST_EMAIL = process.env.E2E_TEST_EMAIL || 'e2e3@test.com';
export const TEST_PASSWORD = process.env.E2E_TEST_PASSWORD || 'e2e123456';
export const TEST_NAME = process.env.E2E_TEST_NAME || 'E2E Test 3';

// Fixed role accounts — seeded by the backend (APP_SEED_EDITOR_* / APP_SEED_STRANGER_*).
// Editor: registered Architect + test space Editor member.
// Stranger: registered Architect, NOT a member of the test space.
export const EDITOR_EMAIL = process.env.APP_SEED_EDITOR_EMAIL || 'test@example.com';
export const EDITOR_PASSWORD = process.env.APP_SEED_EDITOR_PASSWORD || 'testpassword123';
export const STRANGER_EMAIL = process.env.APP_SEED_STRANGER_EMAIL || 'stranger@test.com';
export const STRANGER_PASSWORD = process.env.APP_SEED_STRANGER_PASSWORD || 'stranger123456';
export const STRANGER_NAME = process.env.APP_SEED_STRANGER_NAME || 'Stranger';

// Admin credentials for quota-bypass tests.
export const ADMIN_EMAIL = process.env.APP_SEED_ADMIN_EMAIL || 'admin@test.com';
export const ADMIN_PASSWORD = process.env.APP_SEED_ADMIN_PASSWORD || 'admin123456';

// Test space id — env-driven, mirrors backend migration TEST_SPACE_ID.
export const TEST_SPACE_ID = process.env.E2E_TEST_SPACE_ID || '00000000-0000-0000-0000-000000000010';
export const SPACE_BASE = `/spaces/${TEST_SPACE_ID}/architectures`;

/**
 * Login via the UI. Uses form submit (Enter key) which is more reliable than button click.
 * After login, verifies redirect to the space-scoped value-streams page.
 */
export async function login(page: Page) {
  await loginAs(page, TEST_EMAIL, TEST_PASSWORD);
}

/**
 * Login as the fixed admin account (env-driven; bypasses the 3-space quota).
 */
export async function loginAsAdmin(page: Page) {
  await loginAs(page, ADMIN_EMAIL, ADMIN_PASSWORD);
}

/**
 * Login as the fixed editor account (test space Editor member).
 */
export async function loginAsEditor(page: Page) {
  await loginAs(page, EDITOR_EMAIL, EDITOR_PASSWORD);
}

/**
 * Login as the fixed stranger account (registered, non-member of the test space).
 */
export async function loginAsStranger(page: Page) {
  await loginAs(page, STRANGER_EMAIL, STRANGER_PASSWORD);
}

/**
 * Login as a specific user via the UI (used by ownership tests that need to
 * switch between two space members).
 */
export async function loginAs(page: Page, email: string, password: string) {
  await page.goto('/login');
  await page.fill('input[type="email"]', email);
  await page.fill('input[type="password"]', password);
  await page.press('input[type="password"]', 'Enter');
  // Login success: sidebar visible (environment-agnostic).
  // exact:true — the overview landing page also has entry-card links whose
  // accessible names contain the entity labels (e.g. "价值流 3 …").
  // 30s: cold CI backend (first boot + Argon2 hashing + React hydration) can
  // exceed 10s on loaded runners; longer wait only, no semantic change.
  await expect(page.getByRole('link', { name: '价值流', exact: true })).toBeVisible({ timeout: 30000 });
}

/**
 * Logout via the UI. Architecture pages are public read, so the page stays
 * put and the sidebar switches to the logged-out state (登录 link appears).
 */
export async function logout(page: Page) {
  await page.getByText('退出登录').click();
  await expect(page.getByRole('link', { name: '登录' })).toBeVisible({ timeout: 5000 });
}

/**
 * Ensure user is logged out before test.
 */
export async function ensureLoggedOut(page: Page) {
  await page.goto('/');
  await page.evaluate(() => localStorage.clear());
}
