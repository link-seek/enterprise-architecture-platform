## [Incomplete GraphQL service documentation]
**File:** crates/server/src/graphql/mod.rs:7987-7992
**Severity:** warning
**What is wrong:** The GraphQLService documentation mentions JWT authentication and LifecycleHooks but doesn't explain the three-tier role-based permission system (Admin, Architect, Viewer) or field-level restrictions implemented in entity_guard and field_guard.
**Impact:** Developers reading the code won't understand the complete permission model without reading the entire entity_guard implementation.
**Recommended change:** Expand the documentation to explain: "Queries are public. Mutations require JWT. Roles: Admin (full access), Architect (create/update/delete business entities), Viewer (read-only). Field-level restrictions: password_hash hidden, email admin-only."

## [Undocumented GraphQL endpoint configuration]
**File:** crates/server/src/graphql/mod.rs:8005
**Severity:** suggestion
**What is wrong:** The GraphQL endpoint is hardcoded as "/graphql" but the rationale for this choice and any configuration options are not documented.
**Impact:** Developers need to search the code to find the endpoint location and cannot easily change it.
**Recommended change:** Add a comment explaining the endpoint choice or make it configurable via the config system.

## [Stale comment about replacing auto-generated mutations]
**File:** crates/server/src/graphql/mod.rs:8186
**Severity:** suggestion
**What is wrong:** The comment "These replace seaography's auto-generated CRUD mutations for value_stream" doesn't explain why custom mutations are needed or what additional behavior they provide.
**Impact:** Developers won't understand the design decision or the benefits of custom mutations over auto-generated ones.
**Recommended change:** Expand the comment to explain: "Custom mutations enforce domain invariants (e.g., versioning, lifecycle transitions) that auto-generated CRUD cannot express. They also provide better error messages and validation."

## [Undocumented OWNED_ENTITIES constant]
**File:** crates/server/src/graphql/mod.rs:7817-7821
**Severity:** warning
**What is wrong:** The OWNED_ENTITIES constant lists entities with created_by fields but is never referenced in entity_guard or field_guard, making it dead code or incomplete implementation.
**Impact:** Developers may assume ownership-based authorization is implemented when it's not, leading to security assumptions.
**Recommended change:** Either remove the unused constant or implement ownership checks in entity_guard. If implementing, add documentation explaining the ownership model.

## [Missing documentation for mutation permission checks]
**File:** crates/server/src/graphql/mod.rs:8190-8218
**Severity:** warning
**What is wrong:** The custom valueStreamCreate mutation doesn't document that it bypasses the entity_guard permission checks, making it accessible to all authenticated users including Viewers.
**Impact:** Developers may assume custom mutations have the same permission checks as auto-generated CRUD mutations, leading to security vulnerabilities.
**Recommended change:** Add a comment: "NOTE: This mutation bypasses entity_guard. Permission checks must be added manually by extracting Claims from ctx.data_opt() and verifying role.can_create()."

## [Verbose mutation rejection comment]
**File:** crates/server/src/graphql/mod.rs:8071-8081
**Severity:** suggestion
**What is wrong:** The comment "Fallback: reject mutation requests without JWT" is verbose and the code below it is self-explanatory. The string matching approach is also fragile.
**Impact:** The comment doesn't add value beyond what the code already shows, and the implementation has security implications not mentioned.
**Recommended change:** Replace with: "// Reject mutations without JWT (fragile string check - entity_guard is the authoritative source)."

## [Missing error documentation for parse_importance]
**File:** crates/server/src/graphql/mod.rs:8171-8183
**Severity:** suggestion
**What is wrong:** The parse_importance function doesn't document that it expects PascalCase ("Critical", "High") while the database uses snake_case ("critical", "high"), creating an inconsistency.
**Impact:** Developers won't understand the casing mismatch between GraphQL and REST/DB APIs.
**Recommended change:** Add a comment: "// Note: GraphQL uses PascalCase (Critical, High) while DB/REST uses snake_case (critical, high). Consider standardizing."

## [Incomplete field_guard documentation]
**File:** crates/server/src/graphql/mod.rs:7923-7951
**Severity:** suggestion
**What is wrong:** The field_guard implementation is well-commented with constants, but the function itself lacks a summary comment explaining the overall field-level access control strategy.
**Impact:** Developers need to read the entire function to understand the access control strategy.
**Recommended change:** Add a function-level comment: "// Field-level access control: hides sensitive fields (password_hash) and restricts admin-only fields (email). All other fields are accessible based on entity-level permissions."
