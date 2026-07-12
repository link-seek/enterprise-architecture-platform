# Code Review Synthesis — gitee-1 — 2026-07-12

## Overall Assessment
This PR has one critical security vulnerability that must be fixed before merge: custom ValueStream GraphQL mutations completely bypass authorization, allowing any authenticated user (including Viewers) to create, update, archive, and version value streams. The overall code quality is solid with good DDD layering and domain modeling, but there are several secondary security gaps (unauthenticated reads of sensitive entities, missing JWT expiration validation, OAuth parameter injection) and a few data-integrity risks (missing transaction, missing lifecycle checks) that should also be addressed.

## Critical Issues (must fix before merge)

### 1. Custom ValueStream mutations bypass entity_guard authorization
**File:** `crates/server/src/graphql.rs:395-529`
**Confidence:** high | **Agreement:** 4/7 reviewers (correctness, security, architecture, compatibility)

All four custom mutations (`valueStreamCreate`, `valueStreamUpdate`, `valueStreamArchive`, `valueStreamCreateVersion`) are pushed via `builder.mutations.push()` at lines 428, 466, 493, 528. None of the `FieldFuture` closures extract `Claims` from `ctx.data_opt()` or call `role.can_create()`/`can_update()`/`can_delete()`. The `entity_guard` (lines 51-129) only applies to seaography auto-generated CRUD resolvers, not manually registered mutation fields. Any authenticated user — including those with the `Viewer` role — can execute these mutations, contradicting the authorization model where only `Architect+` can create/update/delete.

**Fix:** Inside each custom mutation's `FieldFuture` closure, extract `Claims` via `ctx.data_opt::<crate::middleware::Claims>()`, return a `FieldError` if absent, then validate `claims.user_role().can_create()` / `can_update()` / `can_delete()` as appropriate. Consider extracting a shared `check_permission(claims, action)` helper that both `entity_guard` and custom mutations call.

## Warnings (should fix)

### 2. Missing transaction in valueStreamCreateVersion
**File:** `crates/business-architecture/src/application/value_stream_service.rs:66-67`
**Confidence:** high | **Agreement:** 2/7 (correctness, architecture)

`create_version` calls `self.repo.save(&current).await?` then `self.repo.save(&new_vs).await?` without a transaction wrapper. If the second save fails after the first succeeds, the current version is persisted as Archived but no new active version exists — an inconsistent state.

**Fix:** Wrap both saves in a database transaction. Accept a transaction context in the service method, or use SeaORM's `DatabaseTransaction` to atomically persist both records.

### 3. No check for archived capability in link_process
**File:** `crates/business-architecture/src/infrastructure/persistence/capability_repo.rs:145-171`
**Confidence:** high | **Agreement:** 2/7 (correctness, architecture)

`link_process` checks `process.status != LifecycleStatus::Active` (line 157) but never loads or checks the capability's status. An archived capability can have new processes linked to it, violating the lifecycle model.

**Fix:** Before inserting the capability_process link, fetch the capability by `capability_id` and verify its status is `Active`, returning `DomainError::CannotReferenceArchived` if not.

### 4. Unauthenticated GraphQL read access to sensitive entities
**File:** `crates/server/src/graphql.rs:64`
**Confidence:** high | **Agreement:** 3/7 (security, architecture, compatibility)

`entity_guard` returns `GuardAction::Allow` unconditionally for `OperationType::Read` (line 64). The `field_guard` only hides `password_hash` (users) and restricts `email` (admin-only). Fields like `token_hash` on `refresh_tokens` and `code_hash` on `oauth_authorization_codes` are fully readable without authentication, exposing `user_id`, `expires_at`, and `revoked_at` alongside the hashes.

**Fix:** At minimum, add `("refresh_tokens", "token_hash")` and `("oauth_authorization_codes", "code_hash")` to `HIDDEN_FIELDS`. Require authentication for reads on `USER_ENTITIES` by checking `Claims` in the `OperationType::Read` branch of `entity_guard`.

### 5. OAuth state parameter injected into redirect URL without encoding
**File:** `crates/user-management/src/infrastructure/http/handlers.rs:460`
**Confidence:** high | **Agreement:** 2/7 (security, architecture)

Line 460 uses `redirect_url.push_str(&format!("&state={}", state))` where `state` comes from user input (`AuthorizeInput.state`). Special characters like `&` or `#` inject additional query parameters or break the URL structure, enabling parameter-injection attacks against the OAuth client.

**Fix:** URL-encode the state value: `redirect_url.push_str(&format!("&state={}", percent_encoding::percent_encode(state.as_bytes(), percent_encoding::NON_ALPHANUMERIC)));`

### 6. verify_jwt does not validate token expiration
**File:** `crates/user-management/src/infrastructure/http/handlers.rs:182-189`
**Confidence:** high | **Agreement:** 1/7 (architecture, citing security)

`verify_jwt` at line 186 uses `&Validation::default()`. In `jsonwebtoken`, `Validation::default()` sets `validate_exp = false`. The `/api/auth/me` handler (line 399) and `/api/auth/role` handler (line 580) both call `verify_jwt`, accepting expired JWTs. By contrast, `jwt_auth_middleware` (middleware.rs:37-38) and `extract_claims_from_headers` (graphql.rs:179-180) both explicitly set `validation.validate_exp = true`.

**Fix:** Replace `Validation::default()` with `Validation::new(jsonwebtoken::Algorithm::HS256)` and set `validation.validate_exp = true`, matching the pattern in `jwt_auth_middleware`.

### 7. Mutation detection via string matching is unreliable
**File:** `crates/server/src/graphql.rs:281`
**Confidence:** high | **Agreement:** 5/7 (architecture, security, maintainability, compatibility, correctness)

`body_str.contains("mutation")` on the raw request body produces false positives (blocking queries with "mutation" in string arguments/comments) and false negatives (aliased mutations bypass the check). This is a fallback defense — `entity_guard` is the authoritative protection for seaography mutations — but future maintainers may mistake it for the security boundary.

**Fix:** Remove the string-based fallback. Rely on `entity_guard` for seaography mutations and add explicit auth checks to custom mutations (see Critical Issue #1). If early-rejection is desired, parse the GraphQL request AST and inspect the operation type.

### 8. Duplicate Claims struct across crate boundaries
**File:** `crates/server/src/middleware.rs:8-14`, `crates/user-management/src/application/token.rs:16-22`
**Confidence:** high | **Agreement:** 3/7 (architecture, maintainability, compatibility)

Both files define identical `Claims` structs with fields `{sub, exp, iat, user_id, role}`. The server crate uses its own `Claims` for GraphQL auth; user-management uses its own for REST auth. They must be kept in sync manually — divergence will cause silent cross-crate auth failures that are hard to trace.

**Fix:** Define `Claims` once in `shared-common` and have both crates import it, or re-export from `user-management` as a public type.

### 9. JWT config field names mismatch HS256 implementation
**File:** `crates/server/src/config.rs:27-28`
**Confidence:** high | **Agreement:** 4/7 (compatibility, architecture, docs, maintainability)

`JwtConfig` fields are named `rsa_private_key_pem` and `rsa_public_key_pem`, implying RSA asymmetric keys. However, all JWT operations use `Algorithm::HS256` with `EncodingKey::from_secret()`/`DecodingKey::from_secret()` (handlers.rs:177, middleware.rs:40, graphql.rs:184). `rsa_public_key_pem` is completely unused in code. Operators who provide RSA PEM keys will get silent failures.

**Fix:** Rename fields to `hmac_secret` or `signing_key` to match the HS256 implementation, and remove the unused `rsa_public_key_pem` field. If renaming is deferred (pre-1.0), add a doc comment clarifying the mismatch.

### 10. OWNED_ENTITIES declared but never used — ownership checks not implemented
**File:** `crates/server/src/graphql.rs:25-29`
**Confidence:** high | **Agreement:** 5/7 (architecture, maintainability, compatibility, docs, correctness)

`OWNED_ENTITIES` is defined at lines 25-29 but never referenced in `entity_guard` (lines 50-129) or `field_guard` (lines 131-159). The ownership-based access control it implies (only owner or admin can modify) is not implemented. `DomainError::NotOwner` exists in the domain layer but is never produced. Any Architect can update/delete any business capability, process, or value stream regardless of ownership. This may be intentional for v0.1.0, but the unused constant is misleading about the security model, and adding ownership checks later would be a silent breaking permission change.

**Fix:** Either implement ownership checks in `entity_guard` for Update/Delete on `OWNED_ENTITIES` (comparing `claims.user_id` against the entity's `created_by`/`owner_id`), or remove the constant and add a TODO comment if ownership is deferred.

### 11. Domain entity → SeaORM model conversion in GraphQL presentation layer
**File:** `crates/server/src/graphql.rs:353-373`
**Confidence:** high | **Agreement:** 2/7 (architecture, maintainability)

`domain_vs_to_model` converts a domain `ValueStream` back to a `value_stream::Model` (SeaORM infrastructure type) so seaography's field resolvers can downcast it. This forces the presentation layer to know about the persistence model shape, breaking the DDD layering that `business-architecture` otherwise maintains.

**Fix:** Move the conversion into `business-architecture::infrastructure` as a `From` impl, or have custom mutations return `FieldValue` directly from domain fields rather than round-tripping through the SeaORM model.

### 12. N+1 query in revoke_all_for_user
**File:** `crates/user-management/src/infrastructure/persistence/auth_repo.rs:87-102`
**Confidence:** high | **Agreement:** 2/7 (performance, maintainability)

Lines 88-92 fetch all tokens with `.all()`, then lines 95-99 loop with individual `.update()` calls (1 SELECT + N UPDATEs). The migration also confirms no index on `user_id`, so even the initial SELECT may scan the full table.

**Fix:** Replace the loop with a single bulk update: `refresh_token::Entity::update_many().filter(Column::UserId.eq(user_id)).filter(Column::RevokedAt.is_null()).col_expr(Column::RevokedAt, Expr::value(Some(now))).exec(&self.db).await`.

### 13. Missing indexes on token_hash and user_id columns
**File:** `crates/user-management/src/infrastructure/persistence/auth_repo.rs:66-72`, `migration/src/m20250101_000002_create_refresh_tokens.rs`
**Confidence:** high | **Agreement:** 1/7 (performance, citing correctness)

`find_by_hash` queries by `token_hash` and `revoke_all_for_user` queries by `user_id`, but the migration creates no indexes on either column — only a primary key on `id`. The `oauth_authorization_codes` table similarly lacks an index on `code_hash` despite `find_by_hash` querying by it. Both tables will do full scans as they grow.

**Fix:** Add migrations creating indexes on `refresh_tokens(token_hash)`, `refresh_tokens(user_id)`, and `oauth_authorization_codes(code_hash)`.

## Suggestions (nice to have)

### Security hardening
- **CorsLayer::permissive() allows all origins** (`crates/server/src/app.rs:108`): Make CORS configurable via `Configuration` — use `permissive()` for local/dev and restrict to specific origins in production.
- **Hardcoded JWT secret in config/local.toml** (`config/local.toml:11`): Replace with a placeholder; document that the JWT secret must be set via `APP__JWT__RSA_PRIVATE_KEY_PEM` env var. Add a startup check that rejects known dev secrets in non-local environments.

### Performance
- **Missing pagination in find_all_versions** (`crates/business-architecture/src/infrastructure/persistence/value_stream_repo.rs:88-99`, `process_repo.rs:90-99`): Both methods call `.all()` with no `.limit()`. Add optional `limit` parameter (default 50) or return paginated results consistent with `list_active`.

### Maintainability
- **Unreachable NoRelation implementation** (`crates/server/src/graphql.rs:308-324`): All three `RelationBuilder` methods call `unreachable!()`. Replace with `unimplemented!("NoRelation: entity has no relations")` to provide context if ever hit, or add a comment documenting the intent.
- **Repeated role check pattern in entity_guard** (`crates/server/src/graphql.rs:66-127`): Three nearly identical blocks for Create/Update/Delete. Extract a `check_write_permission(claims, entity, role_method)` helper.
- **String-based role parsing fallback** (`crates/server/src/middleware.rs:18-19`): `UserRole::from_str(&self.role).unwrap_or(UserRole::Viewer)` silently falls back to Viewer on invalid role strings. Log a `tracing::warn!` when the fallback occurs.
- **UserRole::from_str shadows std::str::FromStr** (`crates/shared-common/src/enums.rs:144-151`): Implement `std::str::FromStr` for `UserRole` instead of a custom `from_str` method, enabling `str.parse::<UserRole>()` and generic contexts.
- **Hardcoded entity table names in guard** (`crates/server/src/graphql.rs:25-46`): Entity table names are hardcoded string literals in `OWNED_ENTITIES`, `USER_ENTITIES`, `HIDDEN_FIELDS`, `ADMIN_ONLY_FIELDS`. Add comments linking each to its SeaORM entity definition to prevent drift.

## Documentation Drift
The following doc/comment updates were identified (suggestion-level):
- [ ] Add comment on `register_value_stream_domain_mutations` noting that custom mutations bypass `entity_guard` and require manual role checks in `crates/server/src/graphql.rs:395`

## Points of Agreement
- **Custom ValueStream mutation auth bypass** (Issue #1): Confirmed by 4 independent reviewers — this is the highest-confidence finding and must be fixed before merge.
- **Mutation detection via string matching** (Issue #7): 5 reviewers flagged this as unreliable; all agree `entity_guard` is the authoritative check and the string fallback should be removed.
- **OWNED_ENTITIES unused** (Issue #10): 5 reviewers confirmed the constant is dead code signaling an incomplete ownership model.
- **JWT config field name mismatch** (Issue #9): 4 reviewers confirmed the RSA-named fields are used as HMAC secrets, creating operational confusion.
- **Duplicate Claims struct** (Issue #8): 3 reviewers confirmed the divergence risk for this security-critical type.

## What Looks Good
- Domain modeling in `business-architecture` follows clean DDD layering with proper separation of domain entities, application services, and infrastructure persistence.
- The `entity_guard` implementation for seaography auto-generated CRUD is well-structured with clear role-based permission checks for Create/Update/Delete operations.
- JWT validation in `jwt_auth_middleware` and `extract_claims_from_headers` correctly sets `validate_exp = true` (the gap is only in the standalone `verify_jwt` helper).
- The `field_guard` correctly hides `password_hash` and restricts `email` to admins.
- OAuth flow implementation is otherwise sound apart from the state encoding issue.
- Configuration system supports environment variable overrides via `config::Environment`, enabling secure production deployment.
