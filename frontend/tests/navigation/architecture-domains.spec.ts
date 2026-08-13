// spec: specs/eap-test-plan.md
// 业务架构与应用架构分域设计：侧边栏分组 + 架构总览页
import { test, expect } from '@playwright/test';
import { login, SPACE_BASE } from '../helpers/auth';

test.describe('架构域分组导航', () => {
  test.beforeEach(async ({ page }) => {
    await login(page);
  });

  test('侧边栏显示业务架构/应用架构分组标题', { tag: '@smoke' }, async ({ page }) => {
    const nav = page.getByRole('navigation');
    await expect(nav.getByRole('heading', { name: '业务架构' })).toBeVisible();
    await expect(nav.getByRole('heading', { name: '应用架构' })).toBeVisible();
  });

  test('业务架构分组下可导航到业务侧页面', { tag: '@smoke' }, async ({ page }) => {
    for (const [label, sub] of [
      ['价值流', 'value-streams'],
      ['业务能力', 'capabilities'],
      ['业务流程', 'processes'],
      ['组织单元', 'organizational-units'],
      ['业务角色', 'business-roles'],
    ] as const) {
      await page.getByRole('link', { name: label, exact: true }).click();
      await expect(page).toHaveURL(`${SPACE_BASE}/${sub}`);
      await expect(page.getByRole('link', { name: label, exact: true })).toHaveClass(/bg-primary/);
    }
  });

  test('应用架构分组下可导航到应用侧页面', { tag: '@smoke' }, async ({ page }) => {
    for (const [label, sub] of [
      ['应用组件', 'applications'],
      ['应用流程', 'application-processes'],
      ['功能模块', 'functional-modules'],
      ['应用接口', 'application-interfaces'],
    ] as const) {
      await page.getByRole('link', { name: label, exact: true }).click();
      await expect(page).toHaveURL(`${SPACE_BASE}/${sub}`);
      await expect(page.getByRole('link', { name: label, exact: true })).toHaveClass(/bg-primary/);
    }
  });

  test('映射关系入口仍可打开', { tag: '@smoke' }, async ({ page }) => {
    await page.getByRole('link', { name: '映射关系' }).click();
    await expect(page).toHaveURL(`${SPACE_BASE}/realizations`);
    await expect(page.getByRole('heading', { name: '映射关系' })).toBeVisible();
  });

  test('架构总览页展示三个区块且可跳转', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/overview`);
    const main = page.locator('main');
    await expect(main.getByRole('heading', { name: '业务架构' })).toBeVisible();
    await expect(main.getByRole('heading', { name: '应用架构' })).toBeVisible();
    await expect(main.getByRole('heading', { name: '跨域支撑' })).toBeVisible();
    // 卡片跳转：业务侧进入价值流页
    await page.getByRole('link', { name: /价值流/ }).first().click();
    await expect(page).toHaveURL(`${SPACE_BASE}/value-streams`);
  });

  test('架构区 index 落至总览页', { tag: '@smoke' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}`);
    await expect(page).toHaveURL(`${SPACE_BASE}/overview`);
  });

  test('业务能力页展示应用支撑弹窗（跨域关联）', { tag: '@regression' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/capabilities`);
    await page.getByRole('button', { name: '应用支撑' }).first().click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await page.getByRole('button', { name: '关闭' }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });

  test('应用流程页展示支撑的业务流程弹窗（跨域关联）', { tag: '@regression' }, async ({ page }) => {
    await page.goto(`${SPACE_BASE}/application-processes`);
    await page.getByRole('button', { name: '支撑业务' }).first().click();
    await expect(page.getByRole('dialog')).toBeVisible();
    await page.getByRole('button', { name: '关闭' }).click();
    await expect(page.getByRole('dialog')).not.toBeVisible();
  });
});
