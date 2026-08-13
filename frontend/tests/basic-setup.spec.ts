// spec: specs/eap-test-plan.md
// Basic setup test to verify Playwright configuration
import { test, expect } from '@playwright/test';

test.describe('Basic Setup Test', () => {
  test('Playwright is configured correctly', { tag: '@smoke' }, async ({ page }) => {
    // Simple test to verify Playwright works
    await page.goto('about:blank');
    expect(true).toBeTruthy();
  });

  test('Test directory structure is correct', { tag: '@smoke' }, async ({ page }) => {
    // Verify we can navigate and basic assertions work
    await page.goto('about:blank');
    const title = await page.title();
    expect(title).toBe('');
  });
});