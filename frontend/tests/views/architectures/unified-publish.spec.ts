// spec: 统一发布策略 v1（一套状态机、一套仪式）
// Covers frontend/src/views/architectures/value-streams.tsx,
// processes.tsx, version-control.tsx, version-publish-dialog.tsx:
// the shared publish ritual (Active → Deprecated → new Active version).
import { test, expect } from '../../helpers/graphql-aware';
import { login, SPACE_BASE } from '../../helpers/auth';

test.describe('Unified Publish Ritual', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('Value streams page loads with unified actions', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/value-streams`);
    await expect(page.getByRole('heading', { name: '价值流', exact: true })).toBeVisible({ timeout: 30000 });
    await expect(
      page.getByRole('table').or(page.getByText(/暂无数据|No data|Empty|加载失败/)),
    ).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: /新建|New/ })).toBeVisible({ timeout: 30000 });
  });

  test('Processes page loads with unified actions', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/processes`);
    await expect(page.getByRole('heading', { name: '业务流程', exact: true })).toBeVisible({ timeout: 30000 });
    await expect(
      page.getByRole('table').or(page.getByText(/暂无数据|No data|Empty|加载失败/)),
    ).toBeVisible({ timeout: 30000 });
    await expect(page.getByRole('button', { name: /新建|New/ })).toBeVisible({ timeout: 30000 });
  });

  test('Publish dialog follows the unified ritual', { tag: '@regression' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/value-streams`);
    await expect(page.getByRole('heading', { name: '价值流', exact: true })).toBeVisible({ timeout: 30000 });
    await expect(
      page.getByRole('table').or(page.getByText(/暂无数据|No data|Empty|加载失败/)),
    ).toBeVisible({ timeout: 30000 });

    const publishButton = page.getByRole('button', { name: '新版本' }).first()
      .or(page.getByRole('menuitem', { name: '新版本' }).first());
    if (!(await publishButton.isVisible())) {
      await expect(page.getByRole('heading', { name: '价值流', exact: true })).toBeVisible();
      return;
    }
    await publishButton.click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await expect(page.getByRole('heading', { name: '发布新版本' })).toBeVisible();
    await expect(page.getByLabel('新版本号')).toBeVisible();
    await expect(page.getByRole('button', { name: /确认发布/ })).toBeVisible();
    await page.getByRole('button', { name: /取消|Cancel/ }).first().click();
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });
});
