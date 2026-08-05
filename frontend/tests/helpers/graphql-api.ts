// Request-based GraphQL helpers for backend-enforcement tests (bypass the UI).
// Resolves relative URLs against the Playwright baseURL; nginx/vite proxy
// `/api/` and `/graphql` to the backend container (see nginx.conf / vite.config.ts).
import { APIRequestContext } from '@playwright/test';

export const SECOND_EDITOR_EMAIL = process.env.E2E_SECOND_EDITOR_EMAIL || 'test@example.com';
export const SECOND_EDITOR_PASSWORD = process.env.E2E_SECOND_EDITOR_PASSWORD || 'testpassword123';
export const ADMIN_EMAIL = process.env.E2E_ADMIN_EMAIL || 'admin@test.com';
export const ADMIN_PASSWORD = process.env.E2E_ADMIN_PASSWORD || 'admin123456';

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
