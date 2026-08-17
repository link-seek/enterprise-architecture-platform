// spec: specs/eap-test-plan.md
import { test, expect } from '../helpers/graphql-aware';
import { login, SPACE_BASE } from '../helpers/auth';

// 使用时间戳后缀保证多次运行不产生同名价值流与阶段，避免列表行/文本匹配歧义
const suffix = Date.now().toString();

test.describe('Value Stream Stages - 阶段管理', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
    await page.goto(`${SPACE_BASE}/value-streams`);
    // 等待列表加载完成（新建价值流按钮可见）后再操作，避免冷启动下点击落空
    await expect(page.getByRole('button', { name: '新建价值流' })).toBeVisible({ timeout: 30000 });
  });

  test('创建价值流后，在详情页添加阶段', { tag: '@regression' }, async ({ page }) => {
    const vsName = `阶段UI测试流${suffix}`;
    const stageName = `需求分析${suffix}`;
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称/ }).fill(vsName);
    await page.getByRole('textbox', { name: /描述/ }).fill('用于阶段UI测试');
    await page.getByRole('button', { name: /创建|保存/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 15000 });

    const row = page.locator('tr').filter({ hasText: vsName });
    await row.getByRole('button', { name: '查看' }).click();
    await expect(page).toHaveURL(/\/value-streams\/.+/, { timeout: 15000 });
    await expect(page.getByRole('heading', { name: '价值流阶段' })).toBeVisible({ timeout: 30000 });

    await page.getByRole('button', { name: /添加阶段/ }).click();
    await page.getByRole('textbox', { name: /阶段名称/ }).fill(stageName);
    await page.getByRole('textbox', { name: /输入/ }).fill('客户需求');
    await page.getByRole('textbox', { name: /输出/ }).fill('需求规格');
    await page.getByRole('button', { name: /创建|保存/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 15000 });
    await expect(page.getByText(stageName)).toBeVisible({ timeout: 15000 });
  });

  test('在详情页编辑和删除阶段', { tag: ['@smoke', '@regression'] }, async ({ page }) => {
    const vsName = `阶段CRUD流${suffix}`;
    const stageName = `设计${suffix}`;
    const editedName = `详细设计${suffix}`;
    await page.getByRole('button', { name: '新建价值流' }).click();
    await page.getByRole('textbox', { name: /名称/ }).fill(vsName);
    await page.getByRole('button', { name: /创建|保存/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 15000 });

    const row = page.locator('tr').filter({ hasText: vsName });
    await row.getByRole('button', { name: '查看' }).click();
    await expect(page).toHaveURL(/\/value-streams\/.+/, { timeout: 15000 });
    await expect(page.getByRole('heading', { name: '价值流阶段' })).toBeVisible({ timeout: 30000 });

    // 添加一个阶段
    await page.getByRole('button', { name: /添加阶段/ }).click();
    await page.getByRole('textbox', { name: /阶段名称/ }).fill(stageName);
    await page.getByRole('button', { name: /创建|保存/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 15000 });
    await expect(page.getByText(stageName)).toBeVisible({ timeout: 15000 });

    // 编辑阶段
    const stageRow = page.locator('tr').filter({ hasText: stageName });
    await stageRow.getByRole('button', { name: /编辑/ }).click();
    await page.getByRole('textbox', { name: /阶段名称/ }).fill(editedName);
    await page.getByRole('button', { name: /保存/ }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible({ timeout: 15000 });
    await expect(page.getByText(editedName)).toBeVisible({ timeout: 15000 });

    // 删除阶段（带二次确认）；断言限定在表格内，避免匹配到确认对话框描述文本
    await page.locator('tr').filter({ hasText: editedName }).getByRole('button', { name: /删除/ }).click();
    await page.getByRole('button', { name: /确认删除/ }).click();
    await expect(page.locator('table').getByText(editedName)).not.toBeVisible({ timeout: 15000 });

    // Reload and re-assert to catch optimistic-update "fake deletes" (#403).
    await page.reload();
    await expect(page.getByRole('heading', { name: '价值流阶段' })).toBeVisible({ timeout: 30000 });
    await expect(page.locator('table').getByText(editedName)).not.toBeVisible({ timeout: 15000 });
  });
});
