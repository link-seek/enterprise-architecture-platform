// spec: specs/eap-test-plan.md
// valueStreamProvisionPilot 发布链路前端 e2e（方案 A：全 mock，不依赖真建仓）。
// page.route 拦截 /graphql 中 valueStreamProvisionPilot，直接返回 success/reused，
// 验证详情页开通按钮走通 + 成功/复用文案 + 输入校验。
import { test, expect } from '../helpers/graphql-aware';
import { login, SPACE_BASE, TEST_EMAIL, TEST_PASSWORD } from '../helpers/auth';
import { cleanupValueStreamsByNamePrefix, findResidualValueStreams } from '../helpers/graphql-api';

const TEST_NAME_PREFIXES = ['开通试点测试_'];

async function selfDeleteAndVerify(
  request: import('@playwright/test').APIRequestContext,
  namePrefixes: string[],
): Promise<void> {
  await cleanupValueStreamsByNamePrefix(request, namePrefixes, TEST_EMAIL, TEST_PASSWORD);
  const residual = await findResidualValueStreams(request, namePrefixes, TEST_EMAIL, TEST_PASSWORD);
  expect(residual).toEqual([]);
}

async function createValueStreamAndGotoDetail(page: import('@playwright/test').Page, name: string) {
  await page.getByRole('button', { name: '新建价值流' }).click();
  await page.getByRole('textbox', { name: /名称|Name/ }).fill(name);
  await page.getByRole('textbox', { name: /描述|Description/ }).fill('开通链路 e2e');
  await page.getByRole('button', { name: /保存|创建|Save|Create/ }).click();
  await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 10000 });
  const row = page.locator('tr').filter({ hasText: name });
  await expect(row).toBeVisible();
  await row.getByRole('button', { name: '查看' }).click();
  await expect(page).toHaveURL(/\/value-streams\/.+/, { timeout: 15000 });
  await expect(page.getByRole('button', { name: '开通试点' })).toBeVisible({ timeout: 30000 });
}

test.describe('Value Stream Provision Pilot - 开通试点（全 mock）', () => {
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

  test('成功开通走通（mock live，非复用）', { tag: '@regression' }, async ({ page, request }) => {
    const name = `开通试点测试_${Date.now()}`;
    await createValueStreamAndGotoDetail(page, name);

    await page.route('**/graphql', async (route) => {
      const post = route.request().postData() ?? '';
      if (post.includes('valueStreamProvisionPilot')) {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            data: {
              valueStreamProvisionPilot: {
                runId: 'run-mock-1',
                status: 'live',
                repoUrl: 'https://github.com/org/pilot-mock',
                prUrl: 'https://github.com/org/pilot-mock/pull/1',
                reused: false,
                templateVersion: 'v1.0.0-pilot',
              },
            },
          }),
        });
      } else {
        await route.continue();
      }
    });

    await page.getByRole('button', { name: '开通试点' }).click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await page.getByPlaceholder('如 v1.0.0-pilot').fill('v1.0.0-pilot');
    await page.getByRole('button', { name: '确认开通' }).click();

    await expect(page.getByText('开通成功，运行实例已上线')).toBeVisible({ timeout: 15000 });
    await expect(page.getByText('仓库 https://github.com/org/pilot-mock')).toBeVisible();
    await expect(page.getByText('PR https://github.com/org/pilot-mock/pull/1')).toBeVisible();

    await page.unroute('**/graphql');
    await selfDeleteAndVerify(request, [name]);
  });

  test('幂等复用文案（mock reused=true）', { tag: '@regression' }, async ({ page, request }) => {
    const name = `开通试点测试_${Date.now()}`;
    await createValueStreamAndGotoDetail(page, name);

    await page.route('**/graphql', async (route) => {
      const post = route.request().postData() ?? '';
      if (post.includes('valueStreamProvisionPilot')) {
        await route.fulfill({
          status: 200,
          contentType: 'application/json',
          body: JSON.stringify({
            data: {
              valueStreamProvisionPilot: {
                runId: 'run-mock-reused',
                status: 'live',
                repoUrl: 'https://github.com/org/pilot-mock',
                prUrl: 'https://github.com/org/pilot-mock/pull/1',
                reused: true,
                templateVersion: 'v1.0.0-pilot',
              },
            },
          }),
        });
      } else {
        await route.continue();
      }
    });

    await page.getByRole('button', { name: '开通试点' }).click();
    await page.getByPlaceholder('如 v1.0.0-pilot').fill('v1.0.0-pilot');
    await page.getByRole('button', { name: '确认开通' }).click();

    await expect(page.getByText('直接复用')).toBeVisible({ timeout: 15000 });

    await page.unroute('**/graphql');
    await selfDeleteAndVerify(request, [name]);
  });

  test('空模板版本前端拦截（不发请求）', { tag: '@regression' }, async ({ page, request }) => {
    const name = `开通试点测试_${Date.now()}`;
    await createValueStreamAndGotoDetail(page, name);

    let publishCalls = 0;
    await page.route('**/graphql', async (route) => {
      const post = route.request().postData() ?? '';
      if (post.includes('valueStreamProvisionPilot')) {
        publishCalls += 1;
      }
      await route.continue();
    });

    await page.getByRole('button', { name: '开通试点' }).click();
    await page.getByRole('button', { name: '确认开通' }).click();
    await expect(page.getByText('请填写模板版本')).toBeVisible({ timeout: 10000 });
    expect(publishCalls).toBe(0);

    await page.unroute('**/graphql');
    await selfDeleteAndVerify(request, [name]);
  });
});
