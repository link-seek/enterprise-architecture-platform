import { test, expect } from '@playwright/test';
import { loginAsStranger, TEST_SPACE_ID } from './helpers/auth';

test('debug stranger detail', async ({ page }) => {
  page.on('response', async (response) => {
    if (!response.url().includes('/graphql')) return;
    const body = await response.json().catch(() => null);
    if (body?.errors) {
      console.log('GRAPHQL ERROR on', response.url());
      console.log(JSON.stringify(body, null, 2));
    }
  });
  await loginAsStranger(page);
  await page.goto('/spaces/' + TEST_SPACE_ID);
  await page.waitForTimeout(3000);
});
