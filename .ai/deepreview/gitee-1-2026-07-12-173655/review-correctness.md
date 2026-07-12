## Missing JWT validation in GraphQL mutation fallback check
**File:** crates/server/src/graphql.rs:8073
**Severity:** critical
**What is wrong:** The fallback mutation rejection checks `body_str.contains("mutation")` as a string match, which can be bypassed by a malformed GraphQL request or by using aliases (e.g., `query { ... }` with embedded mutation via fragment spread in some GraphQL implementations).
**Impact:** An unauthenticated attacker could potentially execute mutations by crafting a request body that doesn't contain the literal string "mutation".
**Recommended change:** Parse the GraphQL request properly and check the operation type, or rely solely on the LifecycleHooks guard which already validates this. Remove the string-based fallback check.

## Missing validation for enum_name in parse_importance
**File:** crates/server/src/graphql.rs:8201
**Severity:** warning
**What is wrong:** `ctx.args.try_get("importance")?.enum_name()?` can return an unexpected enum value that isn't in the match arms of `parse_importance`. The function handles this with an error, but the error message lists expected values while the actual valid GraphQL enum values might differ if the schema allows different casing.
**Impact:** Users may receive confusing error messages when passing valid GraphQL enum values that don't match the expected casing.
**Recommended change:** Normalize the input string (lowercase) before matching, or ensure the GraphQL schema enum values exactly match the match arms.

## No ownership check in entity_guard for Update/Delete
**File:** crates/server/src/graphql.rs:7879-7919
**Severity:** warning
**What is wrong:** The `entity_guard` checks role permissions but never validates ownership for entities in `OWNED_ENTITIES`. An Architect can update/delete any business capability/process/value_stream, even those owned by another user.
**Impact:** Architects can modify resources they don't own, violating the ownership model implied by `created_by`/`owner_id` fields.
**Recommended change:** For Update/Delete on `OWNED_ENTITIES`, add an ownership check: allow if user is Admin OR if `created_by`/`owner_id` matches the authenticated user's ID. This requires fetching the entity to check ownership, which may need a separate hook or query-time filter.

## Claims extraction silently ignores JWT decode errors
**File:** crates/server/src/graphql.rs:7974-7981
**Severity:** suggestion
**What is wrong:** `extract_claims_from_headers` returns `None` on any JWT decode error (expired, invalid signature, malformed). The caller cannot distinguish between "no JWT provided" and "invalid JWT provided".
**Impact:** Security monitoring and debugging are harder; invalid JWTs are treated the same as missing JWTs.
**Recommended change:** Return a `Result<Option<Claims>, JwtError>` to allow callers to log invalid JWTs while still treating them as unauthenticated for authorization purposes.

## Missing transaction in valueStreamCreateVersion
**File:** crates/business-architecture/src/application/value_stream_service.rs:5637-5658
**Severity:** warning
**What is wrong:** `create_version` calls `repo.save(&current).await?` then `repo.save(&new_vs).await?` without a transaction. If the second save fails, the current version is already archived in the database, leaving the value stream in an inconsistent state.
**Impact:** Database inconsistency: the active version is archived but no new version exists, making the logical entity inaccessible.
**Recommended change:** Wrap both saves in a database transaction. The repository trait should support transactional operations, or the service should accept a transaction context.

## Archive operation not idempotent
**File:** crates/business-architecture/src/domain/value_stream/entity.rs:5995-6006
**Severity:** suggestion
**What is wrong:** Calling `archive()` on an already-archived ValueStream returns an `InvalidTransition` error. This makes retry logic harder and can leave the system in an ambiguous state if the caller doesn't know whether the archive succeeded before a network failure.
**Impact:** Retrying archive operations after transient failures will fail, potentially leaving the client unsure of the actual state.
**Recommended change:** Make archive idempotent: if already archived, return `Ok(())` instead of an error. Log a warning for visibility.

## Missing pagination bounds validation
**File:** crates/business-architecture/src/infrastructure/persistence/value_stream_repo.rs:7071-7086
**Severity:** suggestion
**What is wrong:** `list_active` accepts `page` and `per_page` as `u64` without validation. A very large `per_page` could cause memory exhaustion or slow queries.
**Impact:** Denial of service via resource exhaustion.
**Recommended change:** Cap `per_page` at a reasonable maximum (e.g., 100) and return an error or silently clamp if exceeded.

## No check for archived process in link_process
**File:** crates/business-architecture/src/infrastructure/persistence/capability_repo.rs:6300-6326
**Severity:** warning
**What is wrong:** `link_process` checks if the process status is `Active` before linking, but doesn't check if the capability itself is archived. An archived capability can still have processes linked to it.
**Impact:** Inconsistent state: archived capabilities can have new process associations, violating the lifecycle model.
**Recommended change:** Also check that the capability status is `Active` before allowing the link.

## Field guard doesn't check operation type
**File:** crates/server/src/graphql.rs:7923-7951
**Severity:** suggestion
**What is wrong:** `field_guard` takes `_action: OperationType` but ignores it. Hidden fields are blocked for all operations including reads. This is correct for `password_hash`, but the comment suggests field-level write restrictions which aren't implemented.
**Impact:** Field-level write restrictions (e.g., only owner can update `owner_id`) cannot be implemented with the current guard.
**Recommended change:** If field-level write restrictions are needed, use the `action` parameter to differentiate Read vs Update. Otherwise, remove the unused parameter to clarify intent.

## JWT algorithm mismatch risk
**File:** crates/server/src/graphql.rs:7971-7972
**Severity:** warning
**What is wrong:** JWT validation uses `Algorithm::HS256` hardcoded, but the token encoding in `sign_jwt` uses `Header::default()` which may not enforce HS256. If the default header allows a different algorithm, there's a potential algorithm confusion attack.
**Impact:** An attacker could potentially forge tokens if algorithm validation is inconsistent.
**Recommended change:** Explicitly set the algorithm in both encoding and decoding: `Header::new(Algorithm::HS256)` for encoding, and ensure validation matches.

## Missing index on token_hash queries
**File:** crates/user-management/src/infrastructure/persistence/auth_repo.rs:10039-10044
**Severity:** suggestion
**What is wrong:** `find_by_hash` queries `refresh_token` by `token_hash` without an index. As the table grows, this query will become slow.
**Impact:** Performance degradation as refresh tokens accumulate.
**Recommended change:** Add a database index on `token_hash` column via migration.
