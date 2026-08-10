// spec: issue #311 — 空间权限 enforcement E2E 测试（只读，@smoke）
import { test, expect } from '../helpers/graphql-aware';
import { login, loginAsEditor, loginAsStranger, TEST_SPACE_ID, ensureLoggedOut } from '../helpers/auth';

const SPACE_DETAIL_URL = `/spaces/${TEST_SPACE_ID}`;

test.describe('Space Permission Enforcement', () => {
  test('Anonymous can browse public spaces list', { tag: '@smoke' }, async ({ page }) => {
    await ensureLoggedOut(page);
    await page.goto('/spaces');
    await expect(page.getByRole('heading', { name: '所有空间' })).toBeVisible({ timeout: 10000 });

    // Anonymous sees the "登录以编辑" button (prompt to log in).
    await expect(page.getByRole('button', { name: '登录以编辑' })).toBeVisible();

    // Anonymous does NOT see the "创建空间" button.
    await expect(page.getByRole('button', { name: '创建空间' })).not.toBeVisible();
  });

  test('Anonymous can view space detail without edit buttons', { tag: '@smoke' }, async ({ page }) => {
    await ensureLoggedOut(page);
    await page.goto(SPACE_DETAIL_URL);
    await expect(page.getByText('加载中')).not.toBeVisible({ timeout: 10000 });

    // Anonymous sees the "登录以编辑" prompt on the detail page.
    await expect(page.getByRole('button', { name: '登录以编辑' })).toBeVisible();

    // Edit / archive / members buttons are not visible.
    await expect(page.getByRole('button', { name: '编辑', exact: true })).not.toBeVisible();
    await expect(page.getByRole('button', { name: '归档', exact: true })).not.toBeVisible();
    await expect(page.getByRole('button', { name: '成员', exact: true })).not.toBeVisible();
  });

  test('Non-member cannot see edit buttons', { tag: '@smoke' }, async ({ page }) => {
    await loginAsStranger(page);
    await page.goto(SPACE_DETAIL_URL);
    await expect(page.getByText('加载中')).not.toBeVisible({ timeout: 10000 });

    // Non-member does not see edit / archive / members buttons.
    await expect(page.getByRole('button', { name: '编辑', exact: true })).not.toBeVisible();
    await expect(page.getByRole('button', { name: '归档', exact: true })).not.toBeVisible();
    await expect(page.getByRole('button', { name: '成员', exact: true })).not.toBeVisible();
  });

  test('Editor can see edit but not archive/members', { tag: '@smoke' }, async ({ page }) => {
    await loginAsEditor(page);
    await page.goto(SPACE_DETAIL_URL);
    await expect(page.getByText('加载中')).not.toBeVisible({ timeout: 10000 });

    // Editor sees the edit button (canEdit = true).
    await expect(page.getByRole('button', { name: '编辑', exact: true })).toBeVisible({ timeout: 10000 });

    // Editor does NOT see archive or members (owner-only).
    await expect(page.getByRole('button', { name: '归档', exact: true })).not.toBeVisible();
    await expect(page.getByRole('button', { name: '成员', exact: true })).not.toBeVisible();
  });

  test('Owner can see edit/archive/members', { tag: '@smoke' }, async ({ page }) => {
    await login(page);
    await page.goto(SPACE_DETAIL_URL);
    await expect(page.getByText('加载中')).not.toBeVisible({ timeout: 10000 });

    // Owner sees all three action buttons.
    await expect(page.getByRole('button', { name: '编辑', exact: true })).toBeVisible({ timeout: 10000 });
    await expect(page.getByRole('button', { name: '归档', exact: true })).toBeVisible();
    await expect(page.getByRole('button', { name: '成员', exact: true })).toBeVisible();
  });
});