// Request-based GraphQL helpers for backend-enforcement tests (bypass the UI).
// When E2E_API_URL is set (deploy-smoke against prod), requests target the
// backend directly; otherwise they resolve against Playwright baseURL and
// rely on nginx/vite proxy for `/api/` and `/graphql` (see nginx.conf / vite.config.ts).
// Prod frontend is static (no /api proxy), so when E2E_BASE_URL points at a
// static frontend host we derive the backend host instead of hitting it.
import { APIRequestContext } from '@playwright/test';
import { ADMIN_EMAIL, ADMIN_PASSWORD, TEST_EMAIL, TEST_PASSWORD, TEST_SPACE_ID } from './auth';

export function apiUrl(path: string): string {
  const explicit = (process.env.E2E_API_URL ?? '').trim().replace(/\/+$/, '');
  if (explicit) return `${explicit}${path}`;
  const baseUrl = (process.env.E2E_BASE_URL ?? '').trim();
  if (baseUrl.includes('eap.linkseek.net.cn')) return `https://eap-api.linkseek.net.cn${path}`;
  const eapMatch = baseUrl.match(/^(https?:\/\/)eap\.(.+)$/);
  if (eapMatch) return `${eapMatch[1]}eap-api.${eapMatch[2]}${path}`;
  const wwwMatch = baseUrl.match(/^(https?:\/\/)www\.(.+)$/);
  if (wwwMatch) return `${wwwMatch[1]}api.${wwwMatch[2]}${path}`;
  return path;
}

// Single-source credentials from auth.ts (sole owner of E2E credential defaults).
export { ADMIN_EMAIL, ADMIN_PASSWORD, TEST_EMAIL, TEST_PASSWORD, TEST_SPACE_ID };

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
  const res = await request.post(apiUrl('/api/auth/login'), {
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
  const res = await request.post(apiUrl('/graphql'), {
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
 * Fail-closed: GraphQL errors throw instead of returning an empty array.
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
  if (res.errors?.length) {
    throw new Error(`fetchValueStreams failed: ${res.errors.map((e) => e.message).join('; ')}`);
  }
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
  } catch (e) {
    throw new Error(`cleanup login failed for ${email}: ${e instanceof Error ? e.message : String(e)}`);
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
 * Fail-closed: login or query failure throws instead of returning [].
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
  } catch (e) {
    throw new Error(`residual check login failed for ${email}: ${e instanceof Error ? e.message : String(e)}`);
  }
  const streams = await fetchValueStreams(request, token);
  return streams.filter(
    (vs) => vs.id !== SEED_VALUE_STREAM_ID && namePrefixes.some((p) => vs.name.startsWith(p)),
  );
}
