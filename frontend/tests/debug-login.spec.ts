import { test, expect } from '@playwright/test';

test('debug login', { tag: '@smoke' }, async ({ page }) => {
  page.on('console', msg => console.log('CONSOLE:', msg.type(), msg.text()));
  page.on('pageerror', err => console.log('PAGEERROR:', err.message));
  page.on('requestfailed', req => console.log('REQFAIL:', req.url(), req.failure()?.errorText));

  await page.goto('http://localhost:80/login');
  await page.fill('input[type="email"]', 'e2e3@test.com');
  await page.fill('input[type="password"]', 'e2e123456');
  await page.click('button[type="submit"]');
  await page.waitForTimeout(5000);
  console.log('URL after login:', page.url());
  const isLoginHeading = await page.getByRole('heading', { name: '企业架构平台' }).isVisible().catch(() => false);
  console.log('Still on login page:', isLoginHeading);
  const errorText = await page.textContent('body').catch(() => '');
  console.log('Page text contains error:', errorText?.includes('error') || errorText?.includes('失败'));
});