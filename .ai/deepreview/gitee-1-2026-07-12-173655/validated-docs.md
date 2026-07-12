## Missing documentation for custom mutation permission bypass
**File:** crates/server/src/graphql.rs:395-528
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** docs
**Validation notes:** Verified that all four custom ValueStream mutations (valueStreamCreate, valueStreamUpdate, valueStreamArchive, valueStreamCreateVersion) access only `ctx.data::<DatabaseConnection>()` and never check `ctx.data_opt::<Claims>()` for role authorization. The `register_value_stream_domain_mutations` doc comment says "These replace seaography's auto-generated CRUD mutations" but does not note that this replacement bypasses the `GraphqlAuthGuard::entity_guard` role checks. This is a real documentation gap for security-relevant behavior.
**Recommended change:** Add a comment on `register_value_stream_domain_mutations` noting the bypass: "NOTE: These mutations bypass entity_guard. Role checks (can_create/can_update/can_delete) must be added manually inside each FieldFuture by extracting Claims from ctx.data_opt()." Then implement the actual permission checks — documenting a security hole without fixing it is insufficient.

## JWT config field names document wrong algorithm
**File:** crates/server/src/config.rs:27-28
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** compatibility (identified as config mismatch; docs perspective: misleading naming)
**Validation notes:** Verified that `JwtConfig` fields are named `rsa_private_key_pem` and `rsa_public_key_pem` (lines 27-28), implying RSA asymmetric keys. However, the JWT implementation in middleware.rs (line 37) and graphql.rs (line 179) uses `Algorithm::HS256` with `DecodingKey::from_secret()` — HMAC symmetric. The field names are misleading documentation that will cause operators to provide RSA PEM keys and get cryptic failures. The local.toml (line 11) already contains a symmetric secret under the `rsa_private_key_pem` key name.
**Recommended change:** Rename fields to `hmac_secret` or `signing_key` to match the HS256 implementation, or implement RSA/JWT support matching the current field names. If renaming is deferred (pre-1.0), add a doc comment on `JwtConfig`: "Despite the field names, the implementation uses HS256 (HMAC symmetric). The 'rsa_private_key_pem' field is used as the HMAC shared secret."
