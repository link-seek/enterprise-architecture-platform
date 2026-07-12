# Compatibility Review — PR #1: GraphQL role-based permissions + field-level access control

**Project version:** 0.1.0 (pre-1.0, semver exemption applies)
**Diff type:** Initial commit — all files are new (`new file mode 100644`), no prior consumers exist.

> Per semver convention, breaking changes are expected during v0.x.0 development.
> All findings are downgraded to **suggestion** level.

---

## Custom ValueStream mutations bypass entity_guard role checks
**File:** crates/server/src/graphql.rs:8187-8321
**Severity:** suggestion
**What changed:** Custom domain mutations (`valueStreamCreate`, `valueStreamUpdate`, `valueStreamArchive`, `valueStreamCreateVersion`) are registered directly via `builder.mutations.push()` and do not pass through `GraphqlAuthGuard::entity_guard`. The seaography auto-generated CRUD mutations for other entities do go through `entity_guard`, which enforces role checks (Create/Update/Delete require Architect+; user entities require Admin).
**Who breaks:** Any consumer relying on the documented permission model where mutations require specific roles will find ValueStream mutations accessible to any authenticated user (the fallback only checks for a valid JWT, not role).
**Recommended change:** Either implement role checking inside each custom mutation resolver (check `ctx.data_opt::<Claims>()` and validate role before executing), or refactor to use seaography's mutation pipeline so `entity_guard` applies uniformly.

## JWT config field names mismatch implementation algorithm
**File:** crates/server/src/config.rs:7743-7748
**Severity:** suggestion
**What changed:** `JwtConfig` fields are named `rsa_private_key_pem` and `rsa_public_key_pem`, implying RSA (asymmetric) key pairs. However, the JWT implementation in `middleware.rs` and `graphql.rs` uses `jsonwebtoken` with `Algorithm::HS256` (HMAC symmetric) and `DecodingKey::from_secret()`. The "private key PEM" field is actually used as an HMAC shared secret.
**Who breaks:** Once deployed, any operator who provides actual RSA PEM-formatted keys (as the field name suggests) will get silent failures or cryptic JWT errors. Renaming these fields later would be a breaking config change for deployed instances.
**Recommended change:** Rename to `secret_key` (for HS256) or implement actual RSA/JWT support matching the field names. If staying with HS256, rename fields to `hmac_secret` or `signing_key` to match reality.

## GraphQL read operations are unauthenticated
**File:** crates/server/src/graphql.rs:7856
**Severity:** suggestion
**What changed:** `entity_guard` returns `GuardAction::Allow` unconditionally for `OperationType::Read`. Combined with `field_guard` only blocking `password_hash` (all roles) and `email` (non-admin), all other entity data — including user names, roles, statuses, timestamps, and all business architecture data — is readable without any authentication.
**Who breaks:** If this public-read contract ships and consumers build clients assuming unauthenticated reads, restricting read access later (e.g., requiring JWT for user entity reads) would break those clients.
**Recommended change:** If public reads are intentional for business entities, consider at minimum requiring authentication for user-management entity reads (`users`, `refresh_tokens`, `oauth_authorization_codes`). Document the public-read policy explicitly.

## Fragile mutation detection via string matching
**File:** crates/server/src/graphql.rs:8072-8081
**Severity:** suggestion
**What changed:** When no JWT is present, the fallback auth check uses `body_str.contains("mutation")` to decide whether to reject the request. This string match is fragile: a GraphQL query containing the word "mutation" in a comment, string argument, or field alias would be incorrectly rejected; conversely, a mutation sent via GET or with unusual formatting could bypass the check.
**Who breaks:** Clients sending queries with "mutation" in string literals or comments will get spurious 401 errors. Fixing this later to use proper AST parsing would change rejection behavior.
**Recommended change:** Parse the request as a GraphQL document and check the operation type from the AST, or rely solely on the `entity_guard` LifecycleHooks for mutation authorization (which already handles this correctly for seaography mutations).

## UserRole::from_str shadows std::str::FromStr convention
**File:** crates/shared-common/src/enums.rs:8690-8697
**Severity:** suggestion
**What changed:** `UserRole` defines a custom `from_str(&str) -> Option<Self>` method instead of implementing the standard `FromStr` trait (which returns `Result<Self, E>`). This shadows the naming convention for `FromStr` without providing the standard trait, preventing use with `str.parse::<UserRole>()`.
**Who breaks:** Consumers expecting the standard `FromStr` trait (e.g., for use with `parse()`, `?` operator, or generic code) will not find it. Adding `FromStr` later alongside the existing method would be additive, but removing the custom `from_str` to replace it with the trait would be breaking.
**Recommended change:** Implement `std::str::FromStr` for `UserRole` (returning the invalid string as the error), and either deprecate or remove the custom `from_str` method. The `FromStr` trait integrates with Rust's `str::parse()` and the `?` operator.

## OWNED_ENTITIES constant declared but unused
**File:** crates/server/src/graphql.rs:7817-7821
**Severity:** suggestion
**What changed:** `OWNED_ENTITIES` lists `business_capabilities`, `business_processes`, and `value_streams` but is never referenced in `entity_guard` or `field_guard`. This suggests ownership-based (row-level) access control was planned but not implemented.
**Who breaks:** If ownership checks are added later (e.g., only the creator can update their own resources), this would change the effective permissions for non-admin users who previously could update any resource — a silent behavior change.
**Recommended change:** Either implement ownership-based filtering in `entity_guard` (check `created_by`/`owner_id` against the authenticated user's ID for Update/Delete on owned entities), or remove the unused constant to avoid misleading future readers.
