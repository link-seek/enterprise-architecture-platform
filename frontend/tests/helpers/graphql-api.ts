// Request-based GraphQL helpers for backend-enforcement tests (bypass the UI).
// Resolves relative URLs against the Playwright baseURL; nginx/vite proxy
// `/api/` and `/graphql` to the backend container (see nginx.conf / vite.config.ts).
import { APIRequestContext } from '@playwright/test';

export const SECOND_EDITOR_EMAIL = process.env.E2E_SECOND_EDITOR_EMAIL || process.env.APP_SEED_EDITOR_EMAIL || 'test@example.com';
export const SECOND_EDITOR_PASSWORD = process.env.E2E_SECOND_EDITOR_PASSWORD || process.env.APP_SEED_EDITOR_PASSWORD || 'testpassword123';
export const ADMIN_EMAIL = process.env.E2E_ADMIN_EMAIL || process.env.APP_SEED_ADMIN_EMAIL || 'admin@test.com';
export const ADMIN_PASSWORD = process.env.E2E_ADMIN_PASSWORD || process.env.APP_SEED_ADMIN_PASSWORD || 'admin123456';
export const TEST_EMAIL = process.env.E2E_TEST_EMAIL || process.env.SMOKE_TEST_EMAIL || process.env.APP_SEED_ADMIN_EMAIL || 'e2e3@test.com';
export const TEST_PASSWORD = process.env.E2E_TEST_PASSWORD || process.env.SMOKE_TEST_PASSWORD || process.env.APP_SEED_ADMIN_PASSWORD || 'e2e123456';
export const TEST_SPACE_ID = process.env.E2E_TEST_SPACE_ID || '00000000-0000-0000-0000-000000000010';

export interface GqlResponse {
  data?: Record<string, any>
  errors?: Array<{ message: string; path?: string[] }>
}

export interface ApiSession {
  token: string
  userId: string
}

/** Login via the auth API. Returns the bearer token plus the user id. */
export async function apiLogin(
  request: APIRequestContext,
  email: string,
  password: string,
): Promise<ApiSession> {
  const res = await request.post('/api/auth/login', {
    data: { email, password },
    headers: { 'Content-Type': 'application/json' },
  });
  const body = await res.json();
  const token = body?.access_token ?? body?.token ?? body?.accessToken;
  if (!token) {
    throw new Error(`Login failed for ${email}: ${JSON.stringify(body)}`);
  }
  return {
    token: token as string,
    userId: body?.user?.id as string,
  };
}

/** Execute a GraphQL operation as `token`. Returns parsed JSON body. */
export async function gql(
  request: APIRequestContext,
  token: string,
  query: string,
): Promise<GqlResponse> {
  const res = await request.post('/graphql', {
    data: { query },
    headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
  });
  return (await res.json()) as GqlResponse;
}

/**
 * Teardown helper: soft-delete all value streams in the test space whose name
 * starts with one of the given prefixes. Used in `afterAll` to prevent e2e
 * test data from accumulating across runs (the root cause of production data
 * pollution). Only deletes rows the caller's token is allowed to delete (owner
 * or admin); errors are swallowed so a missing row never fails the suite.
 */
export async function cleanupValueStreamsByNamePrefix(
  request: APIRequestContext,
  namePrefixes: string[],
  email: string = TEST_EMAIL,
  password: string = TEST_PASSWORD,
): Promise<void> {
  let token: string;
  try {
    const session = await apiLogin(request, email, password);
    token = session.token;
  } catch {
    return; // cannot login → nothing to clean
  }
  const res = await gql(
    request,
    token,
    `{ valueStreamsBySpace(spaceId: "${TEST_SPACE_ID}") { id name } }`,
  );
  const streams = res.data?.valueStreamsBySpace ?? [];
  for (const vs of streams) {
    if (namePrefixes.some((p) => vs.name.startsWith(p))) {
      try {
        await gql(request, token, `mutation { valueStreamDelete(id: "${vs.id}") }`);
      } catch {
        // Swallow delete errors so cleanup never fails the suite.
      }
    }
  }
}
