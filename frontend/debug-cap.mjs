import { chromium } from '@playwright/test';

const BASE = 'http://localhost:80';
const SPACE_ID = '00000000-0000-0000-0000-000000000010';

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();
await page.goto(BASE + '/login');
await page.fill('input[type="email"]', 'e2e3@test.com');
await page.fill('input[type="password"]', 'e2e123456');
await page.press('input[type="password"]', 'Enter');
await page.waitForTimeout(2000);
console.log('URL after login:', page.url());

// Go to capabilities page
await page.goto(BASE + `/spaces/${SPACE_ID}/architectures/capabilities`);
await page.waitForTimeout(3000);

// Dump the buttons in the row containing 更新前名称
const row = page.locator('tr').filter({ hasText: '更新前名称' });
console.log('row count:', await row.count());
if (await row.count() > 0) {
  const btns = row.locator('button');
  console.log('buttons in row:', await btns.count());
  for (let i = 0; i < await btns.count(); i++) {
    console.log('btn', i, 'html:', (await btns.nth(i).evaluate(el => el.outerHTML)).slice(0, 400));
  }
  // Check for data-icon svgs in the row
  const icons = row.locator('svg');
  console.log('svgs in row:', await icons.count());
  for (let i = 0; i < await icons.count(); i++) {
    console.log('svg', i, 'outerHTML:', (await icons.nth(i).evaluate(el => el.outerHTML)).slice(0, 300));
  }
}

// Also dump the first row's actions
const firstRow = page.locator('tr').first();
console.log('first row html:', (await firstRow.evaluate(el => el.outerHTML)).slice(0, 800));

await browser.close();
