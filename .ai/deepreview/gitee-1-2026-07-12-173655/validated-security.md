## Custom ValueStream mutations bypass entity_guard authorization
**File:** crates/server/src/graphql.rs:395-528
**Severity:** critical
**Confidence:** high
**Original reviewer(s):** security, architecture, compatibility
**Validation notes:** Verified in source: all four custom mutations (`valueStreamCreate`, `valueStreamUpdate`, `valueStreamArchive`, `valueStreamCreateVersion`) are pushed via `builder.mutations.push()` without any role check. None of the `FieldFuture` closures call `ctx.data_opt::<Claims>()` or validate `role.can_create()`/`can_update()`/`can_delete()`. The `entity_guard` only applies to seaography auto-generated CRUD, not custom mutations. Any authenticated user (including Viewer) can execute these mutations.
**Recommended change:** Add an explicit role check inside each custom mutation's `FieldFuture` closure: extract `Claims` via `ctx.data_opt::<Claims>()`, return an error if absent, then call `claims.user_role().can_create()` / `can_update()` / `can_delete()` as appropriate. Return a `FieldError` if unauthorized.

## Unauthenticated GraphQL read access to sensitive entities
**File:** crates/server/src/graphql.rs:63-64
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** security, compatibility
**Validation notes:** Verified: `entity_guard` returns `GuardAction::Allow` unconditionally for `OperationType::Read` (line 64). The `refresh_tokens` and `oauth_authorization_codes` entities are registered (lines 550-551) and queryable without authentication. While `token_hash`/`code_hash` are SHA-256 hashes (not reversible to actual tokens), exposing them alongside `user_id`, `expires_at`, and `revoked_at` is information leakage. The `field_guard` does not block `token_hash` or `code_hash` — only `password_hash` is hidden.
**Recommended change:** At minimum, require authentication for reads on `USER_ENTITIES` (refresh_tokens, oauth_authorization_codes, users). Add a check in `entity_guard` for `OperationType::Read` on `USER_ENTITIES` that requires valid Claims.

## OAuth state parameter injected into redirect URL without encoding
**File:** crates/user-management/src/infrastructure/http/handlers.rs:458-461
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** security
**Validation notes:** Verified: `state` from user-controlled `AuthorizeInput` is interpolated via `format!("&state={}", state)` without URL-encoding. An attacker can inject additional query parameters (e.g., `state=foo&evil=injected`) into the OAuth redirect URL, enabling parameter-injection attacks against the OAuth client.
**Recommended change:** URL-encode the `state` value: `redirect_url.push_str(&format!("&state={}", percent_encoding::percent_encode(state.as_bytes(), percent_encoding::NON_ALPHANUMERIC)));`

## CorsLayer::permissive() allows all origins
**File:** crates/server/src/app.rs:108
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** security
**Validation notes:** Verified: `CorsLayer::permissive()` is hardcoded in `build_router` with no environment-based override. While acceptable for local development, this allows any origin to make cross-origin requests to authenticated endpoints in any deployment. The config system supports environment-specific settings but CORS is not configurable.
**Recommended change:** Make CORS configurable via `Configuration`: use `CorsLayer::permissive()` for local/dev environments and restrict to specific origins in production. Example: `CorsLayer::new().allow_origin(state.config.server.allowed_origins.iter().map(|o| o.parse().unwrap()).collect::<Vec<_>>())`.

## Hardcoded JWT secret committed in config/local.toml
**File:** config/local.toml:11
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** security
**Validation notes:** Verified: `rsa_private_key_pem = "dev-secret-key-for-local-testing-32ch"` is committed to version control. The config system does support environment variable override via `config::Environment` (config.rs:62), so production deployments can use `APP__JWT__RSA_PRIVATE_KEY_PEM`. However, the dev secret is permanently exposed in git history and could be accidentally used in deployed environments.
**Recommended change:** Remove the hardcoded value from `local.toml` (replace with a placeholder or empty string) and document that the JWT secret must be set via the `APP__JWT__RSA_PRIVATE_KEY_PEM` environment variable. Add a startup check that rejects known dev secrets in non-local environments.

## Mutation detection via string matching is unreliable
**File:** crates/server/src/graphql.rs:280-289
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** security, correctness, architecture, maintainability
**Validation notes:** Verified: `body_str.contains("mutation")` on the raw request body is a heuristic that produces both false positives (blocking queries with "mutation" in string arguments/comments) and false negatives (mutations with unusual formatting bypass the check). However, this is a fallback defense — the `entity_guard` is the authoritative protection for seaography mutations. For custom ValueStream mutations (which bypass entity_guard), this string check is the only unauthenticated-access defense, but the real fix is adding role checks to those mutations (finding #1 above).
**Recommended change:** Remove the string-based fallback check. Instead, after parsing the `async_graphql::Request`, inspect the operation type from the parsed AST, or rely solely on `entity_guard` for seaography mutations and add explicit auth checks to custom mutations.

## No ownership check in entity_guard for Update/Delete
**File:** crates/server/src/graphql.rs:25-29, 87-127
**Severity:** warning
**Confidence:** medium
**Original reviewer(s):** correctness, architecture, compatibility
**Validation notes:** Verified: `OWNED_ENTITIES` is defined (lines 25-29) but never referenced in `entity_guard` (lines 51-129). Any Architect can update/delete any business capability, process, or value stream regardless of ownership. The `DomainError::NotOwner` variant exists in the domain layer but is never produced. This may be intentional for v0.1.0 (small-team model where all Architects share access), but the unused constant is misleading about the intended security model.
**Recommended change:** Either implement ownership checks in `entity_guard` for Update/Delete on `OWNED_ENTITIES` (comparing `claims.user_id` against the entity's `created_by`/`owner_id`), or remove the `OWNED_ENTITIES` constant and add a TODO comment if ownership is deferred to a later version.
