import { chromium } from '@playwright/test';

const browser = await chromium.launch({ headless: true });
const page = await browser.newPage();
await page.goto('http://localhost/login');
await page.fill('#email', 'e2e3@test.com');
await page.fill('#password', 'e2e123456');
await page.click('button[type="submit"]');
await page.waitForTimeout(3000);
const url = page.url();
console.log('URL:', url);
await page.goto('http://localhost/spaces/00000000-0000-0000-0000-000000000010/architectures/value-streams');
await page.waitForTimeout(3000);
const headings = await page.locator('h1, h2, h3').allTextContents();
console.log('HEADINGS:', JSON.stringify(headings));
const buttons = await page.getByRole('button').allTextContents();
console.log('BUTTONS:', JSON.stringify(buttons.slice(0, 20)));
await browser.close();
