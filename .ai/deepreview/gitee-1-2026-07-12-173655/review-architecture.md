## Custom ValueStream mutations bypass entity_guard role checks
**File:** crates/server/src/graphql.rs:8187-8321
**Severity:** critical
**What is wrong:** The custom ValueStream domain mutations (`valueStreamCreate`, `valueStreamUpdate`, `valueStreamArchive`, `valueStreamCreateVersion`) are registered via `builder.mutations.push()`, which bypasses seaography's `LifecycleHooks::entity_guard`. The `GraphqlAuthGuard` role-based permission checks (can_create, can_update, can_delete) are never invoked for these mutations — any authenticated user, including Viewers, can perform them.
**Why it matters:** This is a security-critical authorization bypass: the PR's stated goal is role-based permissions, but the custom mutations silently circumvent the guard that enforces them.
**Recommended change:** Either manually invoke the guard logic inside each custom mutation resolver (check `ctx.data_opt::<Claims>()` and validate `role.can_create()` / `can_update()` / `can_delete()`), or use a shared permission-check helper that the guard and custom mutations both call.

## Mutation detection via string matching on request body
**File:** crates/server/src/graphql.rs:8070-8081
**Severity:** critical
**What is wrong:** The fallback mutation rejection checks `body_str.contains("mutation")` on the raw JSON request body. A GraphQL query field named e.g. `mutationStatus` would be falsely rejected, and a mutation sent with unusual whitespace or an alias could potentially bypass the check. The proper approach is to parse the GraphQL request and inspect the operation type.
**Why it matters:** This is a security boundary that relies on string heuristic rather than structured parsing — it will produce both false positives (blocking valid queries) and false negatives (allowing mutations through).
**Recommended change:** After parsing the `async_graphql::Request`, inspect `request.query_type()` or rely solely on the `entity_guard` (which already blocks mutations without Claims) rather than doing pre-execution string matching.

## Duplicate Claims struct across crate boundaries
**File:** crates/server/src/middleware.rs:5-22, crates/user-management/src/application/token.rs:22-37
**Severity:** warning
**What is wrong:** Two identical `Claims` structs exist: one in `server/src/middleware.rs` and one in `user-management/src/application/token.rs`. Both have the same fields (`sub`, `exp`, `iat`, `user_id`, `role`). The server crate uses its own `Claims` for GraphQL auth, while `user-management` uses its own for REST auth — they must be kept in sync manually.
**Why it matters:** Divergence between the two structs will cause silent auth failures or bugs that are hard to trace. This violates DRY across a security-critical type.
**Recommended change:** Define `Claims` once in `shared-common` (or `user-management` as a public re-export) and have both the server and user-management crates use it.

## OWNED_ENTITIES constant declared but never used
**File:** crates/server/src/graphql.rs:7817-7821
**Severity:** warning
**What is wrong:** The `OWNED_ENTITIES` const (`&["business_capabilities", "business_processes", "value_streams"]`) is defined but never referenced in `entity_guard` or `field_guard`. The ownership-based access control it implies (where only the owner or admin can modify) is not implemented.
**Why it matters:** This is dead code that signals an incomplete security model — readers will assume ownership checks exist when they don't. The `DomainError::NotOwner` variant in the domain layer also goes unused.
**Recommended change:** Either implement ownership checks in `entity_guard` for Update/Delete on owned entities (comparing `claims.user_id` against the entity's `created_by`/`owner_id`), or remove the constant and add a TODO if ownership is deferred.

## Domain entity → SeaORM model conversion in GraphQL presentation layer
**File:** crates/server/src/graphql.rs:8145-8165
**Severity:** warning
**What is wrong:** `domain_vs_to_model` converts a domain `ValueStream` back to a `value_stream::Model` (SeaORM infrastructure type) so that seaography's field resolvers can downcast it. This forces the presentation layer (graphql.rs) to know about the infrastructure persistence model, breaking the DDD layering that the `business-architecture` crate otherwise maintains.
**Why it matters:** The domain model's evolution is now coupled to the SeaORM model's shape from outside the crate. Changes to either model require updating this conversion in the wrong layer.
**Recommended change:** Move the `domain_vs_to_model` conversion into `business-architecture::infrastructure` as a `From` impl, or have the custom mutations return `FieldValue` directly from domain fields rather than round-tripping through the SeaORM model.

## NoRelation phantom type with unreachable!() panics
**File:** crates/server/src/graphql.rs:8097-8116
**Severity:** suggestion
**What is wrong:** `NoRelation` implements `RelationBuilder` with all three methods calling `unreachable!()`. It exists only to satisfy `register_entity`'s type parameter. If seaography ever calls any of these methods (e.g., when it detects a relation column), the server will panic at runtime.
**Why it matters:** This is a latent panic disguised as a type-system workaround — it trades a compile error for a runtime crash, making the code less safe than it appears.
**Recommended change:** Add a comment documenting that this is intentionally unreachable for entities without relations, or find a seaography API that doesn't require a `RelationBuilder` for relation-less entities.
