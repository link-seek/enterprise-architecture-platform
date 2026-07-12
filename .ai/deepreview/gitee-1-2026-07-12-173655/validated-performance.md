## N+1 query in revoke_all_for_user
**File:** crates/user-management/src/infrastructure/persistence/auth_repo.rs:87-102
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** performance
**Validation notes:** Verified the N+1 pattern: line 88-92 fetches all tokens with `.all()`, then lines 95-99 loop with individual `.update()` calls. The migration confirms no index on `user_id` either, so even the initial SELECT may scan the full table. The fix to use `update_many()` is straightforward and SeaORM supports it.
**Recommended change:** Replace the loop with a single bulk update: `refresh_token::Entity::update_many().filter(Column::UserId.eq(user_id)).filter(Column::RevokedAt.is_null()).col_expr(Column::RevokedAt, Expr::value(Some(now))).exec(&self.db).await`. Also add an index on `user_id` via migration since the initial filter lacks one.

## Missing index on token_hash and user_id columns
**File:** crates/user-management/src/infrastructure/persistence/auth_repo.rs:66-72, migration/src/m20250101_000002_create_refresh_tokens.rs
**Severity:** warning
**Confidence:** high
**Original reviewer(s):** correctness
**Validation notes:** Verified: `find_by_hash` (line 66-72) queries by `token_hash` and `revoke_all_for_user` (line 88) queries by `user_id`, but the migration (m20250101_000002) creates no indexes on either column — only a primary key on `id`. The `oauth_authorization_codes` table similarly lacks an index on `code_hash` despite `find_by_hash` querying by it. Both tables will do full scans as they grow.
**Recommended change:** Add migration creating indexes: `CREATE INDEX idx_refresh_tokens_token_hash ON refresh_tokens(token_hash)` and `CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens(user_id)`. Similarly for `oauth_authorization_codes(code_hash)`.

## Missing pagination in find_all_versions
**File:** crates/business-architecture/src/infrastructure/persistence/value_stream_repo.rs:88-99, crates/business-architecture/src/infrastructure/persistence/process_repo.rs:90-99
**Severity:** suggestion
**Confidence:** high
**Original reviewer(s):** performance
**Validation notes:** Verified: both methods call `.all(&self.db).await?` with no `.limit()`. Value stream repo does apply `.order_by_desc()` but no row cap. Process repo has no ordering either. While business entities rarely exceed 10-20 versions, there is no guard against pathological growth.
**Recommended change:** Add optional `limit` parameter to the repository trait (default e.g. 50) and apply `.limit(limit)` to the query. Alternatively, return a paginated result with `page`/`per_page` parameters consistent with `list_active`.
