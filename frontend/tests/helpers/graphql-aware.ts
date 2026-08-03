// Custom test fixture: auto-fail on GraphQL errors (#293 Part C)
import { test as base, expect } from '@playwright/test';

export const test = base.extend({
  page: async ({ page }, use) => {
    const graphqlErrors: string[] = [];

    page.on('response', async (response) => {
      if (!response.url().includes('/graphql') && !response.url().includes('/api')) return;
      try {
        const body = await response.json();
        if (body?.errors) {
          const msgs = body.errors.map((e: { message: string }) => e.message).join('; ');
          graphqlErrors.push(msgs);
        }
      } catch {
        // Not JSON or already consumed — ignore
      }
    });

    // eslint-disable-next-line react-hooks/rules-of-hooks -- Playwright fixture use, not a React hook
    await use(page);

    if (graphqlErrors.length > 0) {
      throw new Error(`GraphQL errors detected during test:\n${graphqlErrors.join('\n')}`);
    }
  },
});

export { expect };
