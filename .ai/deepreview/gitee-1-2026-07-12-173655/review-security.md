## Custom ValueStream mutations bypass entity_guard authorization
**File:** crates/server/src/graphql.rs:8187-8321
**Severity:** critical
**Type:** security
**What is wrong:** The four custom mutations (`valueStreamCreate`, `valueStreamUpdate`, `valueStreamArchive`, `valueStreamCreateVersion`) are pushed directly onto `builder.mutations` as raw async-graphql dynamic fields. They bypass seaography's `LifecycleHooks` entity_guard entirely, so no role check (can_create, can_update, can_delete) is enforced.
**Attack vector / Impact:** Any authenticated user (including Viewer role) can create, modify, archive, and version value streams, completely bypassing the role-based access control that the PR intends to enforce.
**Recommended change:** Add an explicit role check inside each custom mutation's `FieldFuture` closure by reading `Claims` from `ctx.data_opt::<Claims>()` and calling `claims.user_role().can_create()` / `can_update()` / `can_delete()`, returning a `GuardAction::Block`-style error if unauthorized.

## verify_jwt does not validate token expiration
**File:** crates/user-management/src/infrastructure/http/handlers.rs:9501-9508
**Severity:** warning
**Type:** security
**What is wrong:** `verify_jwt()` uses `Validation::default()` which sets `validate_exp = false`. The `/api/auth/me` and `/api/auth/role` endpoints call this function, so they accept expired JWTs. By contrast, `jwt_auth_middleware` and `extract_claims_from_headers` both explicitly set `validation.validate_exp = true`.
**Attack vector / Impact:** An attacker who obtains an expired JWT can still access `/api/auth/me` and, critically, `/api/auth/role` (the role-escalation endpoint), long after the token should have been invalid.
**Recommended change:** Replace `Validation::default()` with `Validation::new(jsonwebtoken::Algorithm::HS256)` and set `validation.validate_exp = true`, matching the pattern used in `jwt_auth_middleware`.

## Unauthenticated GraphQL read access to sensitive entities
**File:** crates/server/src/graphql.rs:7856
**Severity:** warning
**Type:** security
**What is wrong:** `entity_guard` returns `GuardAction::Allow` for all `OperationType::Read` regardless of authentication or entity type. This exposes `refresh_tokens` (including `token_hash`) and `oauth_authorization_codes` (including `code_hash`) to unauthenticated GraphQL queries.
**Attack vector / Impact:** An unauthenticated attacker can enumerate all refresh token hashes and authorization code hashes via GraphQL queries, aiding offline attacks or session hijacking if a hash is ever used directly.
**Recommended change:** Require authentication for reads on `USER_ENTITIES` (refresh_tokens, oauth_authorization_codes, users), and consider restricting non-owned business entity reads to authenticated users as well.

## OAuth state parameter injected into redirect URL without encoding
**File:** crates/user-management/src/infrastructure/http/handlers.rs:9777-9782
**Severity:** warning
**Type:** security
**What is wrong:** The `state` parameter from user input is interpolated directly into the redirect URL via `format!("&state={}", state)` without URL-encoding. If `state` contains `&`, `#`, or other special characters, it can inject additional query parameters or break the URL structure.
**Attack vector / Impact:** An attacker can craft a `state` value like `foo&evil_param=injected` to inject extra parameters into the redirect URL, potentially confusing the OAuth client or enabling parameter-injection attacks.
**Recommended change:** URL-encode the `state` value before interpolation using `percent_encoding::percent_encode(state.as_bytes(), NON_ALPHANUMERIC)`.

## CorsLayer::permissive() allows all origins
**File:** crates/server/src/app.rs:7679
**Severity:** suggestion
**Type:** security
**What is wrong:** `CorsLayer::permissive()` sets `Access-Control-Allow-Origin: *`, allowing any website to make cross-origin requests to the API including authenticated endpoints.
**Attack vector / Impact:** A malicious website can make authenticated requests if a user's browser has a valid JWT cookie or if the JWT is sent via other browser-accessible means.
**Recommended change:** Restrict allowed origins to the actual frontend domain(s) in production configuration, e.g. `CorsLayer::new().allow_origin(["https://your-app.example.com".parse().unwrap()])`.

## Hardcoded JWT secret committed in config/local.toml
**File:** config/local.toml:5494
**Severity:** suggestion
**Type:** security
**What is wrong:** `rsa_private_key_pem = "dev-secret-key-for-local-testing-32ch"` is a hardcoded JWT signing key committed to version control. While labeled for local testing, if this config is accidentally used in a deployed environment, all tokens are trivially forgeable.
**Attack vector / Impact:** If deployed with this key, any attacker can forge valid JWTs with arbitrary roles (including admin), achieving full account takeover.
**Recommended change:** Remove the hardcoded value; load the JWT secret exclusively from an environment variable (e.g. `APP__JWT__RSA_PRIVATE_KEY_PEM`) which the existing config builder already supports via `config::Environment`.

## Mutation detection via string matching is unreliable
**File:** crates/server/src/graphql.rs:8072-8081
**Severity:** suggestion
**Type:** security
**What is wrong:** The fallback mutation detection uses `body_str.contains("mutation")` on the raw request body. This can false-positive on queries containing "mutation" in string arguments or comments, and can false-negative on mutations using alternative syntax or whitespace. The entity_guard is the real protection, making this check both unreliable and redundant.
**Attack vector / Impact:** Legitimate queries may be rejected (DoS), or the check gives a false sense of security if someone relies on it instead of the entity_guard.
**Recommended change:** Parse the GraphQL request to determine the operation type (query vs mutation) before deciding to reject, or remove this fallback entirely since the entity_guard already blocks unauthenticated mutations at the resolver level.
