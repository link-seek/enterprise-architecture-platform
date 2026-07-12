## Mutation Detection via String Contains
**File:** crates/server/src/graphql.rs:8072
**Severity:** warning
**What is wrong:** The fallback mutation detection uses `body_str.contains("mutation")` which can produce false positives (e.g., a query with a field named "mutation" or a string value containing "mutation").
**Impact:** May incorrectly reject valid queries or allow mutations to slip through in edge cases, creating security risk and debugging difficulty.
**Recommended change:** Parse the GraphQL request to check the actual operation type, or rely solely on the `entity_guard` lifecycle hook which already validates mutations properly.

## Duplicated Claims Struct
**File:** crates/server/src/middleware.rs:8436 and crates/user-management/src/application/token.rs:9055
**Severity:** suggestion
**What is wrong:** Two separate `Claims` structs exist with identical fields — one in `middleware.rs` and one in `application/token.rs`.
**Impact:** Risk of divergence if fields are added to one but not the other; callers must remember which Claims type to use in each context.
**Recommended change:** Define `Claims` once in `shared-common` and reuse across both modules, or at least have `middleware::Claims` wrap the domain type.

## Magic Number for Body Size Limit
**File:** crates/server/src/graphql.rs:8043
**Severity:** suggestion
**What is wrong:** The body size limit `1024 * 1024` (1MB) is hardcoded inline without a named constant or configuration option.
**Impact:** Difficult to adjust for different deployments; the intent is not immediately clear without calculation.
**Recommended change:** Define a constant `MAX_GRAPHQL_BODY_SIZE: usize = 1024 * 1024` or make it configurable via `Configuration`.

## Unreachable NoRelation Implementation
**File:** crates/server/src/graphql.rs:8097-8116
**Severity:** suggestion
**What is wrong:** `NoRelation` implements `RelationBuilder` with `unreachable!()` panics, used only as a placeholder for `RelatedEntityFilter::build`.
**Impact:** If accidentally invoked, the code panics without a helpful message; the pattern is obscure for future maintainers.
**Recommended change:** Add a comment explaining this is a sentinel type for entities without relations, or use `todo!()` with a descriptive message, or use `#[allow(unused)]` and document the intent.

## Hardcoded Entity Table Names in Guard
**File:** crates/server/src/graphql.rs:7817-7838
**Severity:** suggestion
**What is wrong:** Entity table names (`"business_capabilities"`, `"users"`, etc.) are hardcoded as string literals in `OWNED_ENTITIES`, `USER_ENTITIES`, `HIDDEN_FIELDS`, and `ADMIN_ONLY_FIELDS`.
**Impact:** If SeaORM entity table names change, these lists must be manually kept in sync; easy to miss during refactoring.
**Recommended change:** Derive these lists from the SeaORM entity metadata or use type-checked constants generated from the entity definitions.

## Repeated Role Check Pattern
**File:** crates/server/src/graphql.rs:7858-7919
**Severity:** suggestion
**What is wrong:** The `entity_guard` implementation repeats the same pattern for Create/Update/Delete: check claims, check user entity restriction, check role permission.
**Impact:** Three nearly identical code blocks increase maintenance burden and risk of inconsistent fixes.
**Recommended change:** Extract a helper function `check_write_permission(claims, entity, role_method)` that handles the common logic, then call it for each operation type.

## String-Based Role Parsing Fallback
**File:** crates/server/src/middleware.rs:8446
**Severity:** suggestion
**What is wrong:** `Claims::user_role()` uses `UserRole::from_str()` with a fallback to `Viewer` on invalid role strings.
**Impact:** Invalid or corrupted role values in JWTs silently become Viewer, potentially granting less access than intended but hiding the real issue.
**Recommended change:** Log a warning when fallback occurs, or return an error type that forces the caller to handle invalid roles explicitly.

## Inconsistent Error Construction
**File:** crates/user-management/src/infrastructure/http/handlers.rs:9397-9455
**Severity:** suggestion
**What is wrong:** `ApiError` wraps `shared_common::AppError` but has its own `From` implementations for domain errors, while the inner `AppError` also has `From<sea_orm::DbErr>`.
**Impact:** Double-wrapping risk; the conversion logic is split between `ApiError` and `AppError`, making error flow harder to trace.
**Recommended change:** Consolidate error conversions: have `ApiError` directly wrap `DomainError` and `sea_orm::DbErr`, or remove the intermediate `AppError` wrapper for HTTP responses.

## Unused OWNED_ENTITIES Constant
**File:** crates/server/src/graphql.rs:7817
**Severity:** suggestion
**What is wrong:** `OWNED_ENTITIES` is defined but never referenced in the `entity_guard` or `field_guard` implementations.
**Impact:** Dead code that may confuse readers about the intended ownership-based access control design.
**Recommended change:** Either implement ownership-based checks using this list, or remove the unused constant.

## Verbose Repository Save Pattern
**File:** crates/user-management/src/infrastructure/persistence/user_repo.rs:10335-10364
**Severity:** suggestion
**What is wrong:** The `save` method has a large if-else block manually mapping every field between domain entity and SeaORM active model.
**Impact:** Adding a new field requires changes in multiple places (entity, model, save method), increasing risk of missed mappings.
**Recommended change:** Consider deriving a trait for bidirectional conversion, or use a macro to generate the field mapping, or at least group related fields with comments.
