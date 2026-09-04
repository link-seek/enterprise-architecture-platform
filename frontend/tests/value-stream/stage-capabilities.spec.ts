// spec: specs/eap-test-plan.md
// R1：价值流阶段「关联能力」列展示 + 添加/移除关联 + 幂等保存。
// 阶段/能力均为测试自建（owner = 测试账号），后端仅允许实体 owner 或 admin 修改关联。
import { test, expect } from '../helpers/graphql-aware';
import { login, SPACE_BASE, TEST_EMAIL, TEST_PASSWORD, TEST_SPACE_ID } from '../helpers/auth';
import { cleanupValueStreamsByNamePrefix, findResidualValueStreams } from '../helpers/graphql-api';
import type { Page } from '@playwright/test';

// Static prefixes ensure afterAll cleanup catches every run's data; the
// per-test names carry a Date.now() suffix for uniqueness within a run.
const TEST_NAME_PREFIXES = [
  '关联能力测试流_',
  '移除关联流_',
];

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

test.describe('价值流阶段 - 关联能力', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.goto(`${SPACE_BASE}/value-streams`);
    await expect(page.getByRole('button', { name: '新建价值流' })).toBeVisible({ timeout: 30000 });
  });

  test.afterAll(async ({ request }) => {
    await cleanupValueStreamsByNamePrefix(request, TEST_NAME_PREFIXES);
    const residual = await findResidualValueStreams(request, TEST_NAME_PREFIXES);
    expect(residual).toEqual([]);
  });

  test('阶段详情展示关联能力，可勾选新增且幂等保存', { tag: '@regression' }, async ({ page }) => {
    const suffix = Date.now().toString();
    const vsName = `关联能力测试流_${suffix}`;
    const stageName = `测试阶段${suffix}`;
    const capName = `测试能力${suffix}`;

    // 自建价值流 + 阶段（owner = 测试账号，页面按钮对 owner 可见）
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称/ }).fill(vsName);
    await page.getByRole('button', { name: /创建|保存/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 15000 });
    await page.locator('tr').filter({ hasText: vsName }).getByRole('button', { name: '查看' }).click();
    await expect(page.getByRole('heading', { name: '价值流阶段' })).toBeVisible({ timeout: 30000 });
    await page.getByRole('button', { name: /添加阶段/ }).click();
    await page.getByRole('textbox', { name: /阶段名称/ }).fill(stageName);
    await page.getByRole('button', { name: /创建|保存/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 15000 });

    // 自建能力（owner = 测试账号），避免对 seed 能力无写权限
    const token = await apiToken(page);
    const cap = await gh(page, token, `mutation($spaceId: String!) { capabilityCreate(spaceId: $spaceId, name: "${capName}", description: "e2e", level: "l1", maturity: "level2", businessValue: "high") { id name ownerId } }`, { spaceId: TEST_SPACE_ID });
    if (cap.errors) throw new Error(`创建能力失败: ${JSON.stringify(cap.errors)}`);

    // 打开阶段关联能力对话框，勾选新能力并保存
    const stageRow = page.locator('tr').filter({ hasText: stageName });
    await stageRow.getByRole('button', { name: /关联能力/ }).click();
    const dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    const label = dialog.locator('label').filter({ hasText: capName });
    await expect(label).toBeVisible({ timeout: 15000 });
    const checkbox = label.locator('input[type="checkbox"]');
    if (!(await checkbox.isChecked())) {
      await checkbox.check();
    }
    await dialog.getByRole('button', { name: /保存/ }).click();
    await expect(dialog).not.toBeVisible({ timeout: 15000 });

    // 阶段行显示新能力的 Badge
    await expect(page.locator('tr').filter({ hasText: stageName }).getByText(capName).first()).toBeVisible({ timeout: 15000 });

    // 幂等：再次打开并直接保存（不改变勾选），不应产生 GraphQL 错误（fixture 自动断言）
    await page.locator('tr').filter({ hasText: stageName }).getByRole('button', { name: /关联能力/ }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await page.getByRole('dialog').getByRole('button', { name: /保存/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 15000 });
  });

  test('可从阶段移除关联能力', { tag: '@regression' }, async ({ page }) => {
    const suffix = Date.now().toString();
    const vsName = `移除关联流_${suffix}`;
    const stageName = `移除阶段${suffix}`;
    const capName = `待移除能力${suffix}`;

    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称/ }).fill(vsName);
    await page.getByRole('button', { name: /创建|保存/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 15000 });
    await page.locator('tr').filter({ hasText: vsName }).getByRole('button', { name: '查看' }).click();
    await expect(page.getByRole('heading', { name: '价值流阶段' })).toBeVisible({ timeout: 30000 });
    await page.getByRole('button', { name: /添加阶段/ }).click();
    await page.getByRole('textbox', { name: /阶段名称/ }).fill(stageName);
    await page.getByRole('button', { name: /创建|保存/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 15000 });

    const token = await apiToken(page);
    const cap = await gh(page, token, `mutation($spaceId: String!) { capabilityCreate(spaceId: $spaceId, name: "${capName}", description: "e2e", level: "l1", maturity: "level2", businessValue: "high") { id name ownerId } }`, { spaceId: TEST_SPACE_ID });
    if (cap.errors) throw new Error(`创建能力失败: ${JSON.stringify(cap.errors)}`);

    // 先建立关联
    const stageRow = page.locator('tr').filter({ hasText: stageName });
    await stageRow.getByRole('button', { name: /关联能力/ }).click();
    let dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    const label = dialog.locator('label').filter({ hasText: capName });
    await expect(label).toBeVisible({ timeout: 15000 });
    const checkbox = label.locator('input[type="checkbox"]');
    if (!(await checkbox.isChecked())) {
      await checkbox.check();
    }
    await dialog.getByRole('button', { name: /保存/ }).click();
    await expect(dialog).not.toBeVisible({ timeout: 15000 });
    await expect(page.locator('tr').filter({ hasText: stageName }).getByText(capName).first()).toBeVisible({ timeout: 15000 });

    // 取消勾选并保存 → Badge 消失
    await page.locator('tr').filter({ hasText: stageName }).getByRole('button', { name: /关联能力/ }).click();
    dialog = page.getByRole('dialog');
    await expect(dialog).toBeVisible();
    await dialog.locator('label').filter({ hasText: capName }).locator('input[type="checkbox"]').uncheck();
    await dialog.getByRole('button', { name: /保存/ }).click();
    await expect(dialog).not.toBeVisible({ timeout: 15000 });
    await expect(page.locator('tr').filter({ hasText: stageName }).getByText(capName)).not.toBeVisible({ timeout: 15000 });
  });
});
