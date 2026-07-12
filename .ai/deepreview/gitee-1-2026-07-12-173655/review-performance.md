## N+1 query in revoke_all_for_user
**File:** crates/user-management/src/infrastructure/persistence/auth_repo.rs:10060-10075
**Severity:** warning
**Type:** performance
**What is wrong:** `revoke_all_for_user` fetches all unrevoked refresh tokens for a user, then issues a separate `UPDATE` query for each token in a loop. This is N+1: 1 SELECT + N UPDATEs where N is the number of active tokens.
**Impact:** Latency scales linearly with the number of active refresh tokens per user; each iteration is a separate DB round-trip.
**Recommended change:** Replace the loop with a single bulk update: `refresh_token::Entity::update_many().filter(Column::UserId.eq(user_id)).filter(Column::RevokedAt.is_null()).col_expr(Column::RevokedAt, Expr::value(Some(now))).exec(&self.db).await`.

## Per-request DecodingKey allocation in GraphQL JWT extraction
**File:** crates/server/src/graphql.rs:7960-7981
**Severity:** suggestion
**Type:** performance
**What is wrong:** `extract_claims_from_headers` constructs a new `DecodingKey::from_secret()` and `Validation` object on every GraphQL POST request. `DecodingKey::from_secret` allocates and processes the secret bytes each time.
**Impact:** Unnecessary per-request allocation and HMAC key setup on every GraphQL mutation/query, adding minor latency under high load.
**Recommended change:** Pre-compute the `DecodingKey` once in `GraphQLService::new` and store it as a field (or wrap in `Arc`), then pass it into the extraction function instead of the raw secret string.

## LlmBackend reconstructed on every request
**File:** crates/server/src/ai/handlers.rs:7418-7431, crates/server/src/ai/handlers.rs:7441-7470, crates/server/src/app.rs:7692-7713
**Severity:** suggestion
**Type:** performance
**What is wrong:** `LlmBackend::from_config(&state.config.llm)` is called inside `suggest_handler`, `stream_handler`, and `health_handler` on every request, cloning config strings (api_key, model, endpoint) each time.
**Impact:** Per-request string allocations for config that is static after startup; minor overhead that compounds under high request volume.
**Recommended change:** Construct the `LlmBackend` once at startup and store it in `AppState` (or an `Arc<LlmBackend>`), then reference it from handlers instead of rebuilding.

## Missing pagination in find_all_versions
**File:** crates/business-architecture/src/infrastructure/persistence/value_stream_repo.rs:6986-6997, crates/business-architecture/src/infrastructure/persistence/process_repo.rs:6686-6696
**Severity:** suggestion
**Type:** performance
**What is wrong:** Both `find_all_versions` methods load all versions of a logical entity from the database without any limit or pagination. If an entity accumulates many versions over time, this returns the full set in one query.
**Impact:** Memory and latency grow unboundedly with version count; a single GraphQL query could load all historical versions into memory at once.
**Recommended change:** Add optional `limit`/`offset` parameters or a `max_versions` cap to the repository trait and apply `.limit().offset()` to the query, or return a paginated result.
