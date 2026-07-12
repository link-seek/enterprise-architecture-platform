## Mutation Detection via String Contains
**File:** crates/server/src/graphql.rs:281
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** maintainability, security, architecture, compatibility
**Validation notes:** Confirmed: `body_str.contains("mutation")` is fragile and misleading — future maintainers may treat it as the security boundary when `entity_guard` (lines 50-129) is the authoritative check. False positives on queries containing "mutation" in string literals/comments are real.
**Recommended change:** Parse the GraphQL request to check the actual operation type, or rely solely on the `entity_guard` lifecycle hook. At minimum, add a comment clarifying this is a best-effort UX fallback, not a security boundary.

## Domain Entity → SeaORM Model Conversion in Presentation Layer
**File:** crates/server/src/graphql.rs:353-373
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** architecture
**Validation notes:** Confirmed: `domain_vs_to_model` converts a domain `ValueStream` back to a `value_stream::Model` (SeaORM infrastructure type) in the GraphQL presentation layer. This couples the presentation layer to the persistence model shape, violating DDD layering that `business-architecture` otherwise maintains.
**Recommended change:** Move the conversion into `business-architecture::infrastructure` as a `From` impl, or have custom mutations return `FieldValue` directly from domain fields rather than round-tripping through the SeaORM model.

## Duplicated Claims Struct
**File:** crates/server/src/middleware.rs:8-14, crates/user-management/src/application/token.rs:16-22
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** maintainability, architecture
**Validation notes:** Confirmed: Two identical `Claims` structs with fields `sub`, `exp`, `iat`, `user_id`, `role`. Divergence between them would cause silent JWT validation failures that are hard to trace.
**Recommended change:** Define `Claims` once in `shared-common` and reuse across both crates, or have `middleware::Claims` re-export the domain type.

## Unreachable NoRelation Implementation
**File:** crates/server/src/graphql.rs:308-324
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** maintainability, architecture
**Validation notes:** Confirmed: All three `RelationBuilder` methods call `unreachable!()`. If seaography ever invokes any of these methods (e.g., when detecting a relation column), the server panics at runtime with no diagnostic message.
**Recommended change:** Add a comment documenting that this is intentionally unreachable for entities without relations. Consider replacing `unreachable!()` with `unimplemented!("NoRelation: entity has no relations")` to provide context if ever hit.

## Repeated Role Check Pattern in entity_guard
**File:** crates/server/src/graphql.rs:66-127
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** maintainability
**Validation notes:** Confirmed: Three nearly identical code blocks for Create/Update/Delete each check: (1) claims existence, (2) USER_ENTITIES restriction, (3) role permission. Adding a new check requires updating all three blocks.
**Recommended change:** Extract a helper `check_write_permission(claims, entity, role_method)` that handles the common logic, then call it for each operation type.

## String-Based Role Parsing Fallback
**File:** crates/server/src/middleware.rs:18-19
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** maintainability
**Validation notes:** Confirmed: `UserRole::from_str(&self.role).unwrap_or(UserRole::Viewer)` silently falls back to Viewer on invalid role strings. While this is safe (least privilege), it hides corrupted or malicious tokens from monitoring.
**Recommended change:** Log a warning via `tracing::warn!` when the fallback occurs, or return an error type that forces the caller to handle invalid roles explicitly.

## Unused OWNED_ENTITIES Constant
**File:** crates/server/src/graphql.rs:25-29
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** maintainability, architecture, docs, compatibility
**Validation notes:** Confirmed: `OWNED_ENTITIES` is defined but never referenced in `entity_guard` or `field_guard`. Only `USER_ENTITIES` is used. This is dead code that signals an incomplete ownership model.
**Recommended change:** Either implement ownership checks in `entity_guard` for Update/Delete on owned entities, or remove the unused constant and add a TODO comment if ownership is deferred.

## JWT Config Field Names Mismatch Implementation Algorithm
**File:** crates/server/src/config.rs:27-28
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** compatibility
**Validation notes:** Confirmed: `JwtConfig` fields are named `rsa_private_key_pem` and `rsa_public_key_pem`, implying RSA keys. The implementation uses `Algorithm::HS256` (HMAC symmetric) with `EncodingKey::from_secret()` / `DecodingKey::from_secret()`. Operators providing RSA PEM keys will get silent failures.
**Recommended change:** Rename fields to `hmac_secret` or `signing_key` to match the HS256 implementation, or implement RSA/JWT support matching the field names.

## UserRole::from_str Shadows std::str::FromStr Convention
**File:** crates/shared-common/src/enums.rs:144-151
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** compatibility
**Validation notes:** Confirmed: `UserRole` defines `pub fn from_str(&str) -> Option<Self>` instead of implementing the standard `FromStr` trait. This prevents use with `str.parse::<UserRole>()` and the `?` operator in generic contexts.
**Recommended change:** Implement `std::str::FromStr` for `UserRole` (returning the invalid string as the error type), and deprecate the custom `from_str` method.

## N+1 Query Pattern in revoke_all_for_user
**File:** crates/user-management/src/infrastructure/persistence/auth_repo.rs:87-102
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** performance
**Validation notes:** Confirmed: `revoke_all_for_user` fetches all unrevoked tokens then issues individual UPDATE queries in a loop (1 SELECT + N UPDATEs). This is also a maintainability concern — the loop pattern is harder to reason about than a single bulk update.
**Recommended change:** Replace with a single bulk update: `refresh_token::Entity::update_many().filter(Column::UserId.eq(user_id)).filter(Column::RevokedAt.is_null()).col_expr(Column::RevokedAt, Expr::value(Some(now))).exec(&self.db).await`.

## Hardcoded Entity Table Names in Guard
**File:** crates/server/src/graphql.rs:25-46
**Severity:** suggestion
**Confidence:** medium
**Original reviewer(s):** maintainability
**Validation notes:** Plausible: Entity table names are hardcoded as string literals in `OWNED_ENTITIES`, `USER_ENTITIES`, `HIDDEN_FIELDS`, and `ADMIN_ONLY_FIELDS`. If SeaORM entity table names change, these must be manually kept in sync. However, deriving these from SeaORM entity metadata at runtime is impractical with the current API.
**Recommended change:** Add comments linking each string literal to its SeaORM entity definition, or use a build-time code generation step to validate consistency.
