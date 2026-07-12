## Custom ValueStream mutations bypass entity_guard role checks
**File:** crates/server/src/graphql.rs:395-529
**Severity:** critical
**Confidence:** high
**Original reviewer(s):** architecture, security, correctness, compatibility
**Validation notes:** Verified in source: `register_value_stream_domain_mutations` pushes four mutations via `builder.mutations.push()` (lines 428, 466, 493, 528). None of the `FieldFuture` closures extract `Claims` from `ctx.data_opt()` or call `role.can_create()`/`can_update()`/`can_delete()`. The `entity_guard` only applies to seaography-generated CRUD resolvers, not manually pushed mutation fields.
**Recommended change:** Inside each custom mutation's `FieldFuture` closure, extract `Claims` via `ctx.data_opt::<crate::middleware::Claims>()` and validate role permissions before executing domain logic. Return a `FieldError` if unauthorized. Alternatively, extract a shared `check_permission(claims, action)` helper that both `entity_guard` and custom mutations call.

## Mutation detection via string matching on request body
**File:** crates/server/src/graphql.rs:281
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** architecture, security, correctness, compatibility, maintainability
**Validation notes:** Verified: line 281 shows `body_str.contains("mutation")` as a fallback when no JWT is present. This is a defense-in-depth layer, not the primary authorization — `entity_guard` blocks seaography mutations without Claims, and the string check only rejects unauthenticated mutation-shaped requests early. However, it produces both false positives (query fields named "mutationStatus") and false negatives (aliased mutations). Severity downgraded from critical: the entity_guard is the authoritative check for seaography mutations; the real gap is that custom ValueStream mutations have neither guard (see finding above).
**Recommended change:** Remove the string-based fallback entirely and rely on `entity_guard` for seaography mutations. For custom mutations, add explicit role checks (see finding above). If an early-rejection optimization is desired, parse the GraphQL request AST and inspect the operation type.

## verify_jwt does not validate token expiration
**File:** crates/user-management/src/infrastructure/http/handlers.rs:182-189
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** security
**Validation notes:** Verified: `verify_jwt` at line 186 uses `&Validation::default()`. In jsonwebtoken, `Validation::default()` sets `validate_exp = false`. The `/api/auth/me` handler (line 399) and `/api/auth/role` handler (line 580) both call `verify_jwt`, accepting expired JWTs. By contrast, `jwt_auth_middleware` (middleware.rs line 37-38) and `extract_claims_from_headers` (graphql.rs line 179-180) both explicitly set `validation.validate_exp = true`.
**Recommended change:** Replace `Validation::default()` with `Validation::new(jsonwebtoken::Algorithm::HS256)` and set `validation.validate_exp = true`, matching the pattern in `jwt_auth_middleware`.

## Unauthenticated GraphQL read access exposes sensitive entity hashes
**File:** crates/server/src/graphql.rs:64
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** security, compatibility
**Validation notes:** Verified: `entity_guard` line 64 returns `GuardAction::Allow` for all `OperationType::Read`. `HIDDEN_FIELDS` (line 39-41) only blocks `password_hash` on `users`. `ADMIN_ONLY_FIELDS` (line 44-46) only restricts `email` on `users`. The `token_hash` column on `refresh_tokens` and `code_hash` column on `oauth_authorization_codes` are NOT protected by either guard, making them readable by unauthenticated GraphQL queries.
**Recommended change:** At minimum, add `("refresh_tokens", "token_hash")` and `("oauth_authorization_codes", "code_hash")` to `HIDDEN_FIELDS`. Consider also requiring authentication for reads on `USER_ENTITIES` by checking `claims` in the `OperationType::Read` branch.

## Duplicate Claims struct across crate boundaries
**File:** crates/server/src/middleware.rs:7-14, crates/user-management/src/application/token.rs:15-22
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** architecture, maintainability
**Validation notes:** Verified: `middleware::Claims` (lines 7-14) has fields `{sub, exp, iat, user_id, role}`. `token::Claims` (lines 15-22) has identical fields `{sub, exp, iat, user_id, role}`. Both derive `Serialize + Deserialize`. The server crate uses its own `Claims` for GraphQL auth; user-management uses its own for REST auth. They must be kept in sync manually.
**Recommended change:** Define `Claims` once in `shared-common` and have both crates import it. This eliminates the divergence risk for a security-critical type.

## OWNED_ENTITIES constant declared but never used
**File:** crates/server/src/graphql.rs:25-29
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** architecture, correctness, docs, compatibility, maintainability
**Validation notes:** Verified: `OWNED_ENTITIES` is defined at lines 25-29 but never referenced in `entity_guard` (lines 50-129) or `field_guard` (lines 131-159). The ownership-based access control it implies (only owner or admin can modify) is not implemented. `DomainError::NotOwner` likely also goes unused.
**Recommended change:** Either implement ownership checks in `entity_guard` for Update/Delete on owned entities (comparing `claims.user_id` against the entity's `created_by`/`owner_id`), or remove the constant and add a TODO comment if ownership is deferred.

## Domain entity → SeaORM model conversion in GraphQL presentation layer
**File:** crates/server/src/graphql.rs:353-373
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** architecture
**Validation notes:** Verified: `domain_vs_to_model` at lines 353-373 converts a domain `ValueStream` to a `value_stream::Model` (SeaORM infrastructure type) so seaography's field resolvers can downcast it. This forces the presentation layer to know about the persistence model, breaking the DDD layering that `business-architecture` otherwise maintains.
**Recommended change:** Move the conversion into `business-architecture::infrastructure` as a `From` impl, or have custom mutations return `FieldValue` directly from domain fields rather than round-tripping through the SeaORM model.

## Missing transaction in valueStreamCreateVersion
**File:** crates/business-architecture/src/application/value_stream_service.rs:66-67
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** correctness
**Validation notes:** Verified: `create_version` at lines 66-67 calls `self.repo.save(&current).await?` then `self.repo.save(&new_vs).await?` without a transaction wrapper. If the second save fails after the first succeeds, the current version is archived in the DB but no new version exists, leaving the value stream in an inconsistent state.
**Recommended change:** Wrap both saves in a database transaction. The repository trait should support transactional operations, or the service should accept a transaction context (e.g., `DatabaseTransaction`) from the caller.

## No check for archived capability in link_process
**File:** crates/business-architecture/src/infrastructure/persistence/capability_repo.rs:145-171
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** correctness
**Validation notes:** Verified: `link_process` at lines 145-171 checks that the process status is `Active` (line 157) but does NOT check the capability's status. An archived capability can still have new processes linked to it, violating the lifecycle model.
**Recommended change:** Before inserting the link, fetch the capability and verify its status is `Active`. Return `DomainError::CannotReferenceArchived` if the capability is archived.

## OAuth state parameter not URL-encoded in redirect
**File:** crates/user-management/src/infrastructure/http/handlers.rs:460
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** security
**Validation notes:** Verified: line 460 shows `redirect_url.push_str(&format!("&state={}", state))` — the `state` value from user input is interpolated directly into the redirect URL without URL-encoding. Characters like `&`, `#`, or `%` in `state` can inject additional query parameters or break the URL structure.
**Recommended change:** URL-encode the `state` value before interpolation: `redirect_url.push_str(&format!("&state={}", percent_encoding::percent_encode(state.as_bytes(), percent_encoding::NON_ALPHANUMERIC)));`

## JWT config field names mismatch HS256 implementation
**File:** crates/server/src/config.rs:27-28
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** compatibility
**Validation notes:** Verified: `JwtConfig` fields are named `rsa_private_key_pem` and `rsa_public_key_pem` (lines 27-28), implying RSA asymmetric keys. However, the implementation uses `Algorithm::HS256` (HMAC symmetric) throughout — `EncodingKey::from_secret()` in handlers.rs line 177, `DecodingKey::from_secret()` in middleware.rs line 40 and graphql.rs line 184. The "private key PEM" field is actually used as an HMAC shared secret. Operators who provide RSA PEM keys will get silent failures.
**Recommended change:** Rename fields to `hmac_secret` or `signing_key` to match the HS256 implementation. Alternatively, implement RSA/JWT support matching the field names.
