// spec: specs/eap-test-plan.md
// R3/R4：发布新版本 → 旧版进入 deprecated 兼容期，弹窗列出受影响能力。
// 通过 GraphQL API 自建能力并建立 能力↔流程 关系（owner = 测试账号），
// 再走 UI 发布，验证 affectedLinks 弹窗与 deprecated 标记。
import { test, expect } from '../helpers/graphql-aware';
import { login, SPACE_BASE, TEST_EMAIL, TEST_PASSWORD, TEST_SPACE_ID } from '../helpers/auth';
import type { Page } from '@playwright/test';

const suffix = Date.now().toString();

async function apiToken(page: Page): Promise<string> {
  const res = await page.request.post('/api/auth/login', {
    data: { email: TEST_EMAIL, password: TEST_PASSWORD },
  });
  const body = await res.json();
  const token = body?.access_token ?? body?.accessToken;
  if (!token) throw new Error(`登录接口未返回 token: ${JSON.stringify(body).slice(0, 200)}`);
  return token as string;
}

async function gh(page: Page, token: string, query: string, variables?: Record<string, unknown>) {
  const res = await page.request.post('/graphql', {
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
    data: { query, variables },
  });
  return res.json();
}

test.describe('业务流程 - 发布新版本（Deprecated 生命周期）', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.goto(`${SPACE_BASE}/processes`);
    await expect(page.getByRole('button', { name: '新建流程' })).toBeVisible({ timeout: 30000 });
  });

  test('发布新版本后旧版标记 deprecated，并展示受影响能力', { tag: '@regression' }, async ({ page }) => {
    // 新建流程（创建者即 owner，发布按钮仅对 owner 可见）
    const name = `发布版本流程_${suffix}`;
    const capName = `发布关联能力_${suffix}`;
    await page.getByRole('button', { name: '新建流程' }).click();
    await page.getByRole('textbox', { name: /名称/ }).fill(name);
    await page.getByRole('button', { name: /创建/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 15000 });
    const row = page.locator('tr').filter({ hasText: name });
    await expect(row).toBeVisible({ timeout: 15000 });

    // 通过 API 找到新流程 id，自建能力并建立 能力↔流程 关系（双方 owner 均为测试账号）
    const token = await apiToken(page);
    const list = await gh(page, token, `query($spaceId: String!) { businessProcessesBySpace(spaceId: $spaceId) { id logicalId name status } }`, { spaceId: TEST_SPACE_ID });
    const created = (list.data?.businessProcessesBySpace ?? []).find((p: { name: string }) => p.name === name);
    if (!created) throw new Error(`未找到新建流程 ${name}`);
    const cap = await gh(page, token, `mutation($spaceId: String!) { capabilityCreate(spaceId: $spaceId, name: "${capName}", description: "e2e", level: "l1", maturity: "level2", businessValue: "high") { id name ownerId } }`, { spaceId: TEST_SPACE_ID });
    if (cap.errors) throw new Error(`创建能力失败: ${JSON.stringify(cap.errors)}`);
    const capId = cap.data.capabilityCreate.id as string;
    const link = await gh(page, token, `mutation($capabilityId: String!, $processId: String!) { capabilityProcessCreate(capabilityId: $capabilityId, processId: $processId) { capabilityId processId } }`, { capabilityId: capId, processId: created.id });
    if (link.errors) throw new Error(`关联能力失败: ${JSON.stringify(link.errors)}`);

    // 点击「发布新版本」
    await page.locator('tr').filter({ hasText: name }).getByRole('button', { name: '发布新版本' }).click();
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible({ timeout: 15000 });
    // 弹窗列出受影响能力
    await expect(dialog.getByText(capName)).toBeVisible({ timeout: 15000 });

    // 确认发布
    await dialog.getByRole('button', { name: /确认发布/ }).click();
    // 发布成功弹窗展示受影响关系（名称 + 新旧版本）
    await expect(dialog.getByText(/发布成功/)).toBeVisible({ timeout: 15000 });
    await expect(dialog.getByText(capName)).toBeVisible();
    await dialog.getByRole('button', { name: /关闭/ }).click();
    await expect(dialog).not.toBeVisible({ timeout: 15000 });

    // 列表中旧版本行出现 deprecated 标记
    await expect(page.getByText('deprecated').first()).toBeVisible({ timeout: 15000 });
  });
});
