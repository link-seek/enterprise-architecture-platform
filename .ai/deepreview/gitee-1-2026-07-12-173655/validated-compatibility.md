## Custom ValueStream mutations bypass entity_guard role checks
**File:** crates/server/src/graphql.rs:395-529
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** compatibility, architecture, security
**Validation notes:** Verified: all four custom mutations (`valueStreamCreate`, `valueStreamUpdate`, `valueStreamArchive`, `valueStreamCreateVersion`) are pushed via `builder.mutations.push()` and contain no Claims extraction or role check inside their `FieldFuture` closures. The string fallback at line 281 only requires a valid JWT, not a specific role. Any authenticated Viewer can execute these mutations, contradicting the entity_guard model where only Architect+ can create/update/delete. Fixing this later by adding role checks would break clients that assumed Viewer access.
**Recommended change:** Add `let claims = ctx.data_opt::<crate::middleware::Claims>().ok_or_else(|| async_graphql::Error::new("Authentication required"))?;` and `if !claims.user_role().can_create() { return Err(...); }` inside each custom mutation resolver, matching the entity_guard pattern.

## JWT config field names mismatch implementation algorithm
**File:** crates/server/src/config.rs:27-28
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** compatibility, architecture, correctness
**Validation notes:** Verified: `JwtConfig` fields are `rsa_private_key_pem` and `rsa_public_key_pem` (lines 27-28), implying RSA asymmetric keys. The implementation uses `Algorithm::HS256` with `EncodingKey::from_secret()` (handlers.rs:177) and `DecodingKey::from_secret()` (middleware.rs:40, graphql.rs:184). `rsa_public_key_pem` is completely unused in code — only defined in the config struct and TOML files. Renaming these fields later would be a breaking config change for deployed instances.
**Recommended change:** Rename to `hmac_secret` or `signing_key` to match the HS256 implementation, and remove the unused `rsa_public_key_pem` field. If RSA support is planned, track it separately.

## GraphQL read operations are unauthenticated
**File:** crates/server/src/graphql.rs:64
**Severity:** suggestion
**Confidence:** medium
**Original reviewer(s):** compatibility, security
**Validation notes:** Verified: `OperationType::Read => GuardAction::Allow` at line 64 grants unauthenticated read access to all entities. `field_guard` restricts `password_hash` (all roles) and `email` (non-admin), but all other data is publicly readable. This is a deliberate design choice, but it creates a public-read API contract: restricting reads later (e.g., requiring JWT for user entities) would break unauthenticated clients. The security exposure of `refresh_tokens`/`oauth_authorization_codes` is a separate concern.
**Recommended change:** If public reads are intentional for business entities, require authentication at minimum for `USER_ENTITIES` reads and document the public-read policy explicitly to set consumer expectations.

## Fragile mutation detection via string matching
**File:** crates/server/src/graphql.rs:281
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** compatibility, security, correctness, maintainability
**Validation notes:** Verified: `body_str.contains("mutation")` at line 281. A GraphQL query containing "mutation" in a string argument, comment, or field alias would be falsely rejected with 401. The `entity_guard` is the authoritative protection for seaography mutations, making this fallback both redundant and unreliable. Fixing to use proper AST parsing would change rejection behavior for edge-case inputs.
**Recommended change:** Parse the GraphQL request and inspect the operation type from the AST, or remove this fallback entirely since `entity_guard` already blocks unauthenticated mutations at the resolver level.

## OWNED_ENTITIES constant declared but unused
**File:** crates/server/src/graphql.rs:25-29
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** compatibility, architecture, correctness, docs, maintainability
**Validation notes:** Verified: `OWNED_ENTITIES` is defined at lines 25-29 but never referenced in `entity_guard` (lines 50-129) or `field_guard` (lines 131-159). `DomainError::NotOwner` exists in the domain layer (error.rs:16) but is never produced. If ownership checks are added later, non-admin users who could previously update any resource would lose access — a silent breaking permission change.
**Recommended change:** Either implement ownership checks in `entity_guard` for Update/Delete on owned entities, or remove the unused constant and add a TODO comment if ownership is deferred.

## Duplicate Claims struct across crate boundaries
**File:** crates/server/src/middleware.rs:8-14, crates/user-management/src/application/token.rs:16-22
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** architecture, maintainability
**Validation notes:** Verified: both structs have identical fields (`sub: String`, `exp: usize`, `iat: usize`, `user_id: uuid::Uuid`, `role: String`). The server crate's `Claims` is used for GraphQL auth; the user-management crate's `Claims` is used for REST auth. They must be kept in sync manually. Divergence would cause silent cross-crate auth failures. Consolidating into `shared-common` later could require re-exports that change public API surfaces.
**Recommended change:** Define `Claims` once in `shared-common` and re-export from both crates, or have `middleware::Claims` wrap the `user-management` type via a newtype or Deref.
