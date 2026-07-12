## Custom ValueStream mutations bypass entity_guard role checks
**File:** crates/server/src/graphql.rs:395-529
**Severity:** critical
**Confidence:** high
**Original reviewer(s):** correctness, security, architecture, compatibility
**Validation notes:** Verified in source: the four custom mutations (valueStreamCreate, valueStreamUpdate, valueStreamArchive, valueStreamCreateVersion) are pushed via `builder.mutations.push()` at lines 428, 466, 493, 528. None of the FieldFuture closures extract Claims or check role permissions. The entity_guard (lines 51-129) only applies to seaography auto-generated resolvers, not to these manually registered mutations.
**Recommended change:** Inside each custom mutation's FieldFuture closure, extract `Claims` via `ctx.data_opt::<crate::middleware::Claims>()` and return an error if missing or if `claims.user_role().can_create()`/`can_update()`/`can_delete()` returns false.

## Missing transaction in valueStreamCreateVersion
**File:** crates/business-architecture/src/application/value_stream_service.rs:66-67
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** correctness
**Validation notes:** Verified: `create_version` calls `self.repo.save(&current).await?` then `self.repo.save(&new_vs).await` without wrapping in a transaction. If the second save fails after the first succeeds, the current version is persisted as Archived but no new active version exists — an inconsistent state.
**Recommended change:** Wrap both saves in a database transaction. Either accept a transaction context in the service method, or use SeaORM's `DatabaseTransaction` to atomically persist both the archived current and the new version.

## No check for archived capability in link_process
**File:** crates/business-architecture/src/infrastructure/persistence/capability_repo.rs:145-171
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** correctness
**Validation notes:** Verified: `link_process` at line 157 checks `process.status != LifecycleStatus::Active` but never loads or checks the capability's status. An archived capability can have new processes linked to it, which is inconsistent with the lifecycle model that blocks linking to archived processes.
**Recommended change:** Before inserting the capability_process link, also fetch the capability by `capability_id` and verify its status is `Active`, returning `DomainError::CannotReferenceArchived` if not.

## Unauthenticated GraphQL read access to sensitive entities
**File:** crates/server/src/graphql.rs:64
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** security
**Validation notes:** Verified: `entity_guard` returns `GuardAction::Allow` unconditionally for `OperationType::Read` at line 64. Combined with `field_guard` only hiding `password_hash` (users) and restricting `email` (admin-only), fields like `token_hash` on `refresh_tokens` and `code_hash` on `oauth_authorization_codes` are fully readable without authentication.
**Recommended change:** Require authentication for reads on `USER_ENTITIES` (refresh_tokens, oauth_authorization_codes, users). At minimum, add a Claims check for Read on these entities in `entity_guard`.

## OAuth state parameter injected into redirect URL without encoding
**File:** crates/user-management/src/infrastructure/http/handlers.rs:460
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** security
**Validation notes:** Verified: line 460 uses `redirect_url.push_str(&format!("&state={}", state))` where `state` comes from user input (`input.state`). Special characters like `&` or `#` in the state value will inject extra query parameters or break the URL structure.
**Recommended change:** URL-encode the state value before interpolation: `use percent_encoding::{percent_encode, NON_ALPHANUMERIC}; redirect_url.push_str(&format!("&state={}", percent_encode(state.as_bytes(), NON_ALPHANUMERIC)));`

## Duplicate Claims struct across crate boundaries
**File:** crates/server/src/middleware.rs:8-14, crates/user-management/src/application/token.rs:16-22
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** architecture, maintainability
**Validation notes:** Verified: both files define identical `Claims` structs with the same fields (sub, exp, iat, user_id, role). The server crate uses its own Claims for GraphQL auth; user-management uses its own for REST auth. Divergence will cause silent auth failures.
**Recommended change:** Define `Claims` once in `shared-common` and have both crates import it, or re-export from `user-management` as a public type.

## JWT config field names misrepresent HS256 implementation
**File:** crates/server/src/config.rs:27-28
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** compatibility
**Validation notes:** Verified: `JwtConfig` fields are `rsa_private_key_pem` and `rsa_public_key_pem`, implying RSA keys. However, all JWT operations use `Algorithm::HS256` with `EncodingKey::from_secret()`/`DecodingKey::from_secret()` (handlers.rs:174-177, middleware.rs:40, graphql.rs:182-184). The field names will mislead operators into providing RSA PEM keys.
**Recommended change:** Rename fields to `hmac_secret` or `signing_key` to match the HS256 implementation, or implement RSA/JWT support matching the current field names.
