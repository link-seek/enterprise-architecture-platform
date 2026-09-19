// spec: issue #311 — 空间权限 enforcement E2E 测试（只读，@smoke）
import { test, expect } from '../helpers/graphql-aware';
import { login, loginAsEditor, loginAsStranger, TEST_EMAIL, TEST_PASSWORD, TEST_SPACE_ID, STRANGER_EMAIL, ensureLoggedOut } from '../helpers/auth';
import { apiLogin, gql } from '../helpers/graphql-api';

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

  test('Non-member cannot see edit buttons', { tag: '@smoke' }, async ({ page, request }) => {
    // Isolation: members.spec.ts (@regression) adds the stranger as editor and
    // may leave the membership behind. Remove via API (deterministic, no dialog
    // flakiness) so the stranger is a guaranteed non-member regardless of test
    // order / reruns. Failures here are non-fatal: UI assertions below decide.
    try {
      const owner = await apiLogin(request, TEST_EMAIL, TEST_PASSWORD);
      const lookup = await gql(
        request,
        owner.token,
        `{ spaceUserByEmail(spaceId: "${TEST_SPACE_ID}", email: "${STRANGER_EMAIL}") { id } }`,
      );
      const strangerId = (lookup.data?.spaceUserByEmail as { id: string } | null)?.id;
      if (strangerId) {
        await gql(
          request,
          owner.token,
          `mutation { spaceRemoveMember(spaceId: "${TEST_SPACE_ID}", userId: "${strangerId}") }`,
        );
      }
    } catch {
      // Best-effort cleanup; the assertions below are authoritative.
    }

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