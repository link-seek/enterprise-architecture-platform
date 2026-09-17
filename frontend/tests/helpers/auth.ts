// Shared test helpers for E2E tests
import { Page, expect } from '@playwright/test';

// Test credentials — env-driven for multi-environment reuse
// Defaults work for local dev and CI integration (localhost backend seeded
// with e2e3/test@example.com). SMOKE_TEST_* / APP_SEED_ADMIN_* fallbacks are
// only for external deploy-smoke (E2E_BASE_URL set, e.g. production where the
// e2e3 account is never seeded): applying them against a localhost backend
// would hijack the login with prod credentials that don't exist locally.
// See docker-compose.ci.yml (backend seed) — both sides must resolve equally.
const IS_EXTERNAL = !!process.env.E2E_BASE_URL;
const smokeEmail = IS_EXTERNAL ? process.env.SMOKE_TEST_EMAIL : undefined;
const smokePassword = IS_EXTERNAL ? process.env.SMOKE_TEST_PASSWORD : undefined;
const seedAdminEmail = IS_EXTERNAL ? process.env.APP_SEED_ADMIN_EMAIL : undefined;
const seedAdminPassword = IS_EXTERNAL ? process.env.APP_SEED_ADMIN_PASSWORD : undefined;
export const TEST_EMAIL = process.env.E2E_TEST_EMAIL || process.env.APP_SEED_E2E_EMAIL || smokeEmail || seedAdminEmail || 'e2e3@test.com';
export const TEST_PASSWORD = process.env.E2E_TEST_PASSWORD || process.env.APP_SEED_E2E_PASSWORD || smokePassword || seedAdminPassword || 'e2e123456';
export const TEST_NAME = process.env.E2E_TEST_NAME || process.env.APP_SEED_E2E_NAME || 'E2E Test 3';

// Fixed role accounts — seeded by the backend (APP_SEED_EDITOR_* / APP_SEED_STRANGER_*).
// Editor: registered Architect + test space Editor member.
// Stranger: registered Architect, NOT a member of the test space.
// Explicit E2E_* overrides always apply; APP_SEED_* secrets only for external
// deploy-smoke (localhost runs use the compose defaults).
export const EDITOR_EMAIL = process.env.E2E_EDITOR_EMAIL || (IS_EXTERNAL ? process.env.APP_SEED_EDITOR_EMAIL : undefined) || 'test@example.com';
export const EDITOR_PASSWORD = process.env.E2E_EDITOR_PASSWORD || (IS_EXTERNAL ? process.env.APP_SEED_EDITOR_PASSWORD : undefined) || 'testpassword123';
export const STRANGER_EMAIL = process.env.E2E_STRANGER_EMAIL || (IS_EXTERNAL ? process.env.APP_SEED_STRANGER_EMAIL : undefined) || 'stranger@test.com';
export const STRANGER_PASSWORD = process.env.E2E_STRANGER_PASSWORD || (IS_EXTERNAL ? process.env.APP_SEED_STRANGER_PASSWORD : undefined) || 'stranger123456';
export const STRANGER_NAME = process.env.E2E_STRANGER_NAME || (IS_EXTERNAL ? process.env.APP_SEED_STRANGER_NAME : undefined) || 'Stranger';

// Admin credentials — E2E_ADMIN_* always honored (explicit test config);
// SMOKE_TEST_* / APP_SEED_ADMIN_* secrets only for external deploy-smoke.
// Used by tests that need admin-only privileges (e.g. bypassing quota).
export const ADMIN_EMAIL = process.env.E2E_ADMIN_EMAIL || smokeEmail || seedAdminEmail || 'admin@test.com';
export const ADMIN_PASSWORD = process.env.E2E_ADMIN_PASSWORD || smokePassword || seedAdminPassword || 'admin123456';

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
  await expect(page.getByRole('link', { name: '价值流', exact: true })).toBeVisible({ timeout: 10000 });
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
