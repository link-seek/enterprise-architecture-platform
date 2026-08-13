// spec: 匿名用户可查看价值流/业务能力/业务流程（后端已允许匿名读，前端路由放开只读）
import { test, expect } from '../helpers/graphql-aware';
import { ensureLoggedOut, SPACE_BASE } from '../helpers/auth';

test.describe('Anonymous Architecture Access', () => {
  test.beforeEach(async ({ page }) => {
    await ensureLoggedOut(page);
  });

  test('Anonymous user can view value streams list', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/value-streams`);
    await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`);
    await expect(page.getByRole('heading', { name: '价值流' }).first()).toBeVisible({ timeout: 10000 });
    await expect(page.getByText(/加载失败/)).not.toBeVisible({ timeout: 5000 });
  });

  test('Anonymous user can view business capabilities list', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/capabilities`);
    await expect(page).toHaveURL(`${SPACE_BASE}/capabilities`);
    await expect(page.getByRole('heading', { name: '业务能力' }).first()).toBeVisible({ timeout: 10000 });
    await expect(page.getByText(/加载失败/)).not.toBeVisible({ timeout: 5000 });
  });

  test('Anonymous user can view business processes list', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/processes`);
    await expect(page).toHaveURL(`${SPACE_BASE}/processes`);
    await expect(page.getByRole('heading', { name: '业务流程' }).first()).toBeVisible({ timeout: 10000 });
    await expect(page.getByText(/加载失败/)).not.toBeVisible({ timeout: 5000 });
  });

  test('Anonymous user does not see edit buttons', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/value-streams`);
    await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`);
    await expect(page.getByRole('heading', { name: '价值流' }).first()).toBeVisible({ timeout: 10000 });
    // 编辑/新建按钮不应显示给匿名用户
    await expect(page.getByRole('button', { name: /新建|创建|添加/ })).not.toBeVisible();
  });

  test('Anonymous user redirected from users page', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/users`);
    await expect(page).toHaveURL('/login');
  });
});
