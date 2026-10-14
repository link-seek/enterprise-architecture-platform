// spec: specs/eap-test-plan.md
// 统一发布策略 v1：发布新版本（手填版本号）→ 旧版 deprecated + 关联 MOVE 到新版 →
// 成功回执列出已迁移关联；deprecated 旧版可归档。
// 通过 GraphQL API 自建能力并建立 能力↔流程 关系（owner = 测试账号），
// 再走 UI 发布，验证版本号输入、关联迁移、deprecated 标记与归档。
import { test, expect } from '../helpers/graphql-aware';
import { login, SPACE_BASE, TEST_EMAIL, TEST_PASSWORD, TEST_SPACE_ID } from '../helpers/auth';
import { apiUrl } from '../helpers/graphql-api';
import type { Page } from '@playwright/test';

const suffix = Date.now().toString();

async function apiToken(page: Page): Promise<string> {
  const res = await page.request.post(apiUrl('/api/auth/login'), {
    data: { email: TEST_EMAIL, password: TEST_PASSWORD },
  });
  const body = await res.json();
  const token = body?.access_token ?? body?.accessToken;
  if (!token) throw new Error(`登录接口未返回 token: ${JSON.stringify(body).slice(0, 200)}`);
  return token as string;
}

async function gh(page: Page, token: string, query: string, variables?: Record<string, unknown>) {
  const res = await page.request.post(apiUrl('/graphql'), {
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

  test('发布新版本：手填版本号，关联迁移，旧版 deprecated 后可归档', { tag: '@regression' }, async ({ page }) => {
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
    const list = await gh(page, token, `query($spaceId: String!) { businessProcessesBySpace(spaceId: $spaceId) { id logicalId name status businessVersion } }`, { spaceId: TEST_SPACE_ID });
    const created = (list.data?.businessProcessesBySpace ?? []).find((p: { name: string }) => p.name === name);
    if (!created) throw new Error(`未找到新建流程 ${name}`);
    const oldVersion = created.businessVersion as string;
    const cap = await gh(page, token, `mutation($spaceId: String!) { capabilityCreate(spaceId: $spaceId, name: "${capName}", description: "e2e", level: "l1", maturity: "level2", businessValue: "high") { id name ownerId } }`, { spaceId: TEST_SPACE_ID });
    if (cap.errors) throw new Error(`创建能力失败: ${JSON.stringify(cap.errors)}`);
    const capId = cap.data.capabilityCreate.id as string;
    const link = await gh(page, token, `mutation($capabilityId: String!, $processId: String!) { capabilityProcessCreate(capabilityId: $capabilityId, processId: $processId) { capabilityId processId } }`, { capabilityId: capId, processId: created.id });
    if (link.errors) throw new Error(`关联能力失败: ${JSON.stringify(link.errors)}`);

    // 点击「发布新版本」，统一发布弹窗：手填版本号
    await page.locator('tr').filter({ hasText: name }).getByRole('button', { name: '发布新版本' }).click();
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible({ timeout: 15000 });
    // 弹窗列出将被迁移的关联
    await expect(dialog.getByText(capName)).toBeVisible({ timeout: 15000 });
    await dialog.getByRole('textbox', { name: /版本/ }).fill('2.0.0');

    // 确认发布
    await dialog.getByRole('button', { name: /确认发布/ }).click();
    // 发布成功回执列出已迁移关联（名称 + 新旧版本）
    await expect(dialog.getByText(/发布成功/)).toBeVisible({ timeout: 15000 });
    await expect(dialog.getByText(capName)).toBeVisible();
    await dialog.getByRole('button', { name: /关闭/ }).click();
    await expect(dialog).not.toBeVisible({ timeout: 15000 });

    // 旧版本行标记 deprecated，新版本行 active 且为手填版本号
    const oldRow = page.locator('tr').filter({ hasText: name }).filter({ hasText: oldVersion });
    await expect(oldRow.getByText('deprecated')).toBeVisible({ timeout: 15000 });
    const newRow = page.locator('tr').filter({ hasText: name }).filter({ hasText: '2.0.0' });
    await expect(newRow.getByText('active')).toBeVisible({ timeout: 15000 });

    // 关联已 MOVE 到新版：能力的流程关系只剩新版一行
    const rels = await gh(page, token, `query($capabilityId: String!) { capabilityProcessRelations(capabilityId: $capabilityId) { processId businessVersion status valid } }`, { capabilityId: capId });
    if (rels.errors) throw new Error(`查询关系失败: ${JSON.stringify(rels.errors)}`);
    const relations = rels.data?.capabilityProcessRelations ?? [];
    expect(relations).toHaveLength(1);
    expect(relations[0].businessVersion).toBe('2.0.0');
    expect(relations[0].valid).toBe(true);

    // 版本历史弹窗列出新旧两版
    await newRow.getByRole('button', { name: '历史' }).click();
    const historyDialog = page.getByRole('dialog');
    await expect(historyDialog.getByText(oldVersion)).toBeVisible({ timeout: 15000 });
    await expect(historyDialog.getByText('2.0.0')).toBeVisible();
    await historyDialog.getByRole('button', { name: /关闭/ }).click();

    // deprecated 旧版可归档（统一生命周期 Active|Deprecated → Archived）
    await oldRow.getByRole('button', { name: '归档' }).click();
    await expect(oldRow.getByText('archived')).toBeVisible({ timeout: 15000 });
  });
});
