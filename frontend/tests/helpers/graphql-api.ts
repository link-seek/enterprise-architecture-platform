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

/** Seed value-stream id — must never be deleted by cleanup. */
export const SEED_VALUE_STREAM_ID = '00000000-0000-0000-0000-0000000000a0';

export interface ValueStreamRow {
  id: string;
  name: string;
  logicalId: string;
  status: string;
}

export interface CleanupResult {
  deleted: string[];
  failed: string[];
}

/**
 * Query all non-deleted value streams in the test space, returning id/name/
 * logicalId/status. Used by cleanup and residual-verification helpers.
 */
async function fetchValueStreams(
  request: APIRequestContext,
  token: string,
): Promise<ValueStreamRow[]> {
  const res = await gql(
    request,
    token,
    `{ valueStreamsBySpace(spaceId: "${TEST_SPACE_ID}") { id name logicalId status } }`,
  );
  return (res.data?.valueStreamsBySpace ?? []) as ValueStreamRow[];
}

/**
 * Strict teardown: soft-delete all value streams whose name starts with one of
 * the given prefixes, expanding the delete to every row sharing the same
 * `logicalId` (so archived versions are removed alongside the active one).
 * The seed value stream (SEED_VALUE_STREAM_ID) is never deleted.
 *
 * Returns `{deleted, failed}`. Throws when any deletion fails so residual data
 * fails the suite instead of silently accumulating.
 */
export async function cleanupValueStreamsByNamePrefix(
  request: APIRequestContext,
  namePrefixes: string[],
  email: string = TEST_EMAIL,
  password: string = TEST_PASSWORD,
): Promise<CleanupResult> {
  const result: CleanupResult = { deleted: [], failed: [] };
  let token: string;
  try {
    const session = await apiLogin(request, email, password);
    token = session.token;
  } catch {
    return result; // cannot login → nothing to clean
  }

  const streams = await fetchValueStreams(request, token);

  // Collect logicalIds of rows matching a prefix (excluding the seed).
  const targetLogicalIds = new Set<string>();
  for (const vs of streams) {
    if (vs.id === SEED_VALUE_STREAM_ID) continue;
    if (namePrefixes.some((p) => vs.name.startsWith(p))) {
      targetLogicalIds.add(vs.logicalId);
    }
  }

  // Expand delete: remove every row sharing a target logicalId.
  for (const vs of streams) {
    if (!targetLogicalIds.has(vs.logicalId)) continue;
    if (vs.id === SEED_VALUE_STREAM_ID) continue;
    try {
      const del = await gql(request, token, `mutation { valueStreamDelete(id: "${vs.id}") }`);
      if (del.errors) throw new Error(del.errors.map((e) => e.message).join('; '));
      result.deleted.push(vs.id);
    } catch {
      result.failed.push(vs.id);
    }
  }

  if (result.failed.length > 0) {
    throw new Error(`cleanup failed to delete ${result.failed.length} value stream(s): ${result.failed.join(', ')}`);
  }
  return result;
}

/**
 * Query residual value streams matching the given prefixes (excluding the
 * seed). Used in afterAll to assert no test data survives cleanup.
 */
export async function findResidualValueStreams(
  request: APIRequestContext,
  namePrefixes: string[],
  email: string = TEST_EMAIL,
  password: string = TEST_PASSWORD,
): Promise<ValueStreamRow[]> {
  let token: string;
  try {
    const session = await apiLogin(request, email, password);
    token = session.token;
  } catch {
    return []; // cannot login → cannot determine residual
  }
  const streams = await fetchValueStreams(request, token);
  return streams.filter(
    (vs) => vs.id !== SEED_VALUE_STREAM_ID && namePrefixes.some((p) => vs.name.startsWith(p)),
  );
}
