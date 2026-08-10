use std::sync::Arc;

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use argon2::Argon2;
use moka::future::Cache;
use sea_orm::ConnectionTrait;
use sea_orm::DatabaseConnection;
use migration::MigratorTrait;
use shared_common::enums::UserRole;
use uuid::Uuid;
use user_management::domain::user::entity::User;
use user_management::domain::user::repository::UserRepository;
use user_management::infrastructure::persistence::user_repo::SeaOrmUserRepo;

use crate::config::Configuration;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Arc<Configuration>,
    #[allow(dead_code)]
    pub cache: Cache<String, serde_json::Value>,
}

impl AppState {
    pub async fn new(config: Configuration) -> anyhow::Result<Self> {
        // Build connect options so we can enable SQLite foreign keys on *every*
        // pooled connection (a single `PRAGMA foreign_keys=ON` only affects one
        // connection in the pool). `SqliteConnectOptions::foreign_keys(true)`
        // applies the pragma on connect for each connection, so the
        // `ON DELETE CASCADE`/`RESTRICT` constraints declared in migrations
        // (including the new `space_id` foreign keys) are enforced pool-wide.
        let mut opts = sea_orm::ConnectOptions::new(config.database.url.clone());
        opts.sqlx_logging(true);
        if config.database.url.starts_with("sqlite://") {
            opts.map_sqlx_sqlite_opts(|o| o.foreign_keys(true));
        }
        let db = sea_orm::Database::connect(opts).await?;

        // Auto-run migrations on startup
        migration::Migrator::up(&db, None).await?;
        tracing::info!("Database migrations completed successfully");

        // Seed admin account.
        // - If APP_SEED_ADMIN_EMAIL is set, always seed (regardless of APP_ENV).
        // - Otherwise: local/dev use default admin@test.com/admin123456;
        //   production logs a warning prompting the operator to set the env var.
        let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "production".to_string());
        let explicit_email = std::env::var("APP_SEED_ADMIN_EMAIL").ok();
        match explicit_email {
            Some(_) => {
                seed_admin(&db).await?;
            }
            None if app_env.eq_ignore_ascii_case("local") || app_env.eq_ignore_ascii_case("dev") => {
                seed_admin(&db).await?;
            }
            None => {
                tracing::warn!(
                    "No admin seed configured. Set APP_SEED_ADMIN_EMAIL and \
                     APP_SEED_ADMIN_PASSWORD to bootstrap the first admin account."
                );
            }
        }

        // Ensure the seeded "测试空间" (test space) exists and make the admin
        // user its owner so existing/backfilled data has an editable home.
        seed_test_space(&db).await?;

        // Seed fixed role accounts (editor = test space Editor member,
        // stranger = registered non-member) for E2E permission tests. Works in
        // all environments: env set → seed, local/dev → default, production →
        // skip if unset. Idempotent.
        seed_fixed_role_accounts(&db).await?;

        // Dogfood: model the EAP platform's own development flow as a two-layer
        // (business + application) architecture inside the test space. Idempotent.
        crate::seed_dogfood::seed_dogfood(&db).await?;

        let cache = Cache::builder()
            .time_to_live(std::time::Duration::from_secs(300))
            .max_capacity(10_000)
            .build();
        Ok(Self {
            db,
            config: Arc::new(config),
            cache,
        })
    }
}

/// Parse the compile-time `TEST_SPACE_ID` constant into a `Uuid`.
/// Centralised so both `seed_test_space` and `seed_fixed_role_accounts`
/// share a single parse site (and a single panic message if invalid).
fn test_space_uuid() -> Uuid {
    Uuid::parse_str(migration::m20250101_000029_add_space_id_to_business_entities::TEST_SPACE_ID)
        .expect("TEST_SPACE_ID must be a valid UUID")
}

/// Resolve a user by email, creating a new `Architect` account if not found.
///
/// Handles the check-then-act race on `users.email` unique index: if a
/// concurrent insert wins the race, `save` fails with a unique-constraint
/// violation, so we retry `find_by_email` and return the existing user.
/// Returns `(user_id, was_created)` for informational purposes. Callers
/// always grant membership via `upsert_space_member`, which is idempotent
/// and safe for pre-existing users.
async fn resolve_or_create_user(
    repo: &SeaOrmUserRepo,
    email: &str,
    name: &str,
    password: &str,
) -> anyhow::Result<(Uuid, bool)> {
    if let Some(existing) = repo.find_by_email(email).await? {
        return Ok((existing.id, false));
    }
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("password hash error: {e}"))?
        .to_string();
    let user = User::new(
        email.to_string(),
        name.to_string(),
        hash,
        UserRole::Architect,
    );
    match repo.save(&user).await {
        Ok(saved) => Ok((saved.id, true)),
        Err(e) => {
            // Only retry on unique-constraint conflict (concurrent insert
            // race). For other failures (connection lost, disk full, etc.)
            // propagate immediately so we don't mask the real problem.
            // `DomainError::EmailExists` is set by `user_repo::save` when
            // sea-orm's typed `sql_err()` detects a `UniqueConstraintViolation`
            // (portable across SQLite / PostgreSQL / MySQL), avoiding
            // fragile string matching on error messages.
            let is_unique_conflict = matches!(
                &e,
                user_management::domain::error::DomainError::EmailExists
            );
            if !is_unique_conflict {
                return Err(anyhow::anyhow!("failed to create user {email}: {e}"));
            }
            // Race: another instance inserted the same email concurrently.
            // Retry find_by_email; if found, reuse; otherwise propagate.
            match repo.find_by_email(email).await {
                Ok(Some(existing)) => {
                    tracing::warn!(
                        "User {email} was created concurrently; reusing existing account"
                    );
                    Ok((existing.id, false))
                }
                Ok(None) => Err(anyhow::anyhow!("failed to create user {email}: {e}")),
                Err(find_err) => Err(anyhow::anyhow!(
                    "failed to create user {email}: {e}; retry find_by_email also failed: {find_err}"
                )),
            }
        }
    }
}

/// Idempotently upsert a space member row, updating the role if the
/// membership already exists (so e.g. an `editor` is upgraded to `owner`).
/// Protects against accidental downgrade: an existing `owner` is never
/// overwritten by a lower-privilege role (e.g. `editor`).
async fn upsert_space_member(
    db: &DatabaseConnection,
    space_id: &Uuid,
    user_id: Uuid,
    member_role: &str,
) -> anyhow::Result<()> {
    // Whitelist member_role to prevent SQL injection via format!.
    let allowed_roles = ["owner", "editor", "viewer"];
    if !allowed_roles.contains(&member_role) {
        anyhow::bail!("invalid member role: {member_role}");
    }
    let now = chrono::Utc::now().to_rfc3339();
    // Use a parameterised statement so that all values are bound as
    // parameters rather than interpolated into the SQL string. This
    // eliminates SQL-injection risk at the database driver level
    // (space_id / user_id are Uuid blobs, role is a whitelisted string,
    // now is an RFC-3339 timestamp). `Statement::from_sql_and_values`
    // converts `?` placeholders to the backend-specific syntax.
    let stmt = sea_orm::Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"INSERT INTO "space_members" ("space_id","user_id","role","created_at","updated_at")
           VALUES (?, ?, ?, ?, ?)
           ON CONFLICT ("space_id","user_id") DO UPDATE SET
             "role" = CASE WHEN "space_members"."role" = 'owner' AND excluded."role" != 'owner'
                           THEN "space_members"."role"
                           ELSE excluded."role" END,
             "updated_at" = CASE WHEN "space_members"."role" = 'owner' AND excluded."role" != 'owner'
                           THEN "space_members"."updated_at"
                           ELSE excluded."updated_at" END"#,
        [
            sea_orm::Value::Bytes(Some(space_id.as_bytes().to_vec())),
            sea_orm::Value::Bytes(Some(user_id.as_bytes().to_vec())),
            member_role.into(),
            now.clone().into(),
            now.into(),
        ],
    );
    db.execute_raw(stmt)
        .await
        .map_err(|e| anyhow::anyhow!("failed to upsert space member: {e}"))?;
    Ok(())
}

/// Idempotently ensure the test space exists (created by migration with a
/// fixed UUID) and, if an admin user is present, make that admin its owner.
/// Also seeds the E2E test users as space members (editor role) so that
/// integration/E2E tests can exercise the edit path instead of read-only mode.
async fn seed_test_space(db: &DatabaseConnection) -> anyhow::Result<()> {
    let test_space_id = test_space_uuid();

    use sea_orm::FromQueryResult;
    #[derive(FromQueryResult)]
    struct IdRow {
        id: Uuid,
    }

    // Find an admin user to make the space owner (best-effort; if none, the
    // space simply has no owner until one is assigned).
    let admin_id = IdRow::find_by_statement(sea_orm::Statement::from_sql_and_values(
        db.get_database_backend(),
        r#"SELECT "id" FROM "users" WHERE "role" = 'Admin' ORDER BY "created_at" ASC LIMIT 1"#,
        [],
    ))
    .one(db)
    .await?
    .map(|r| r.id);

    if let Some(admin_id) = admin_id {
        upsert_space_member(db, &test_space_id, admin_id, "owner").await?;
    }

    // Seed E2E test owner user and add as space member so that permission
    // tests can exercise the owner path. test@example.com (editor) is seeded
    // by seed_fixed_role_accounts to avoid duplicate seeding. These are only
    // seeded in local/dev environments to avoid leaking test accounts into
    // production.
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "production".to_string());
    if app_env.eq_ignore_ascii_case("local") || app_env.eq_ignore_ascii_case("dev") {
        let test_users = [
            ("e2e3@test.com", "E2E Test 3", "e2e123456", "owner"),
        ];
        let repo = SeaOrmUserRepo::new(db.clone());
        for (email, name, password, member_role) in test_users {
            let (user_id, _) = resolve_or_create_user(&repo, email, name, password).await?;
            upsert_space_member(db, &test_space_id, user_id, member_role).await?;
        }
    }
    Ok(())
}

async fn seed_admin(db: &DatabaseConnection) -> anyhow::Result<()> {
    let email = std::env::var("APP_SEED_ADMIN_EMAIL")
        .unwrap_or_else(|_| "admin@test.com".to_string());
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "production".to_string());
    let is_production = !(app_env.eq_ignore_ascii_case("local") || app_env.eq_ignore_ascii_case("dev"));
    let password = match std::env::var("APP_SEED_ADMIN_PASSWORD") {
        Ok(p) => p,
        Err(_) if is_production => {
            anyhow::bail!(
                "APP_SEED_ADMIN_PASSWORD is required in production-like environments \
                 (APP_ENV='{app_env}'); refusing to seed admin with a default weak password"
            );
        }
        Err(_) => {
            tracing::warn!(
                "APP_SEED_ADMIN_PASSWORD not set, using default test password. \
                 Set this env var in production-like environments."
            );
            "admin123456".to_string()
        }
    };
    let name = std::env::var("APP_SEED_ADMIN_NAME")
        .unwrap_or_else(|_| "Admin".to_string());

    if password.chars().count() < 8 {
        anyhow::bail!("Seed admin password must be at least 8 characters");
    }

    let repo = SeaOrmUserRepo::new(db.clone());
    if repo.find_by_email(&email).await?.is_none() {
        let salt = SaltString::generate(&mut OsRng);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("password hash error: {e}"))?
            .to_string();
        let user = User::new(
            email.clone(),
            name,
            hash,
            UserRole::Admin,
        );
        repo.save(&user).await?;
        tracing::info!("Seeded admin user: {} (password set from config)", email);
    } else {
        tracing::debug!("Seed admin skipped: {} already exists", email);
    }
    Ok(())
}

/// Seed the fixed-role accounts used by E2E permission tests:
/// - **editor**: registered `Architect`, added to the test space as an `Editor`
///   member (can edit content but not manage members/archive the space).
/// - **stranger**: registered `Architect`, **not** a member of any space.
///
/// Mirrors `seed_admin`: env-driven (`APP_SEED_EDITOR_*` / `APP_SEED_STRANGER_*`),
/// idempotent (`find_by_email`), production requires a password ≥ 8 chars, and
/// unset env in production simply skips (zero breakage).
async fn seed_fixed_role_accounts(db: &DatabaseConnection) -> anyhow::Result<()> {
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "production".to_string());
    let is_local = app_env.eq_ignore_ascii_case("local") || app_env.eq_ignore_ascii_case("dev");

    let test_space_id = test_space_uuid();
    let repo = SeaOrmUserRepo::new(db.clone());

    // --- Editor ---
    let editor_email = std::env::var("APP_SEED_EDITOR_EMAIL");
    let editor_password = std::env::var("APP_SEED_EDITOR_PASSWORD");
    match (editor_email, editor_password) {
        (Ok(email), Ok(password)) => {
            if password.chars().count() < 8 {
                anyhow::bail!("APP_SEED_EDITOR_PASSWORD must be at least 8 characters");
            }
            let name =
                std::env::var("APP_SEED_EDITOR_NAME").unwrap_or_else(|_| "Editor".to_string());
            let (user_id, _was_created) =
                resolve_or_create_user(&repo, &email, &name, &password).await?;
            // Always grant test-space membership. The upsert is idempotent and
            // protects against accidental role downgrade (an existing owner is
            // never overwritten by editor). This also recovers from partial
            // failure: if a previous run created the user but membership grant
            // failed, a restart will re-grant membership correctly.
            upsert_space_member(db, &test_space_id, user_id, "editor").await?;
        }
        (Ok(_), Err(_)) | (Err(_), Ok(_)) => {
            tracing::warn!(
                "APP_SEED_EDITOR_EMAIL and APP_SEED_EDITOR_PASSWORD must both be set; \
                 skipping editor seed"
            );
        }
        (Err(_), Err(_)) if is_local => {
            // Local/dev default: test@example.com / testpassword123.
            let (user_id, _) = resolve_or_create_user(
                &repo,
                "test@example.com",
                "测试用户",
                "testpassword123",
            )
            .await?;
            upsert_space_member(db, &test_space_id, user_id, "editor").await?;
        }
        (Err(_), Err(_)) => {
            tracing::debug!("No editor seed configured; skipping (production, env unset)");
        }
    }

    // --- Stranger ---
    let stranger_email = std::env::var("APP_SEED_STRANGER_EMAIL");
    let stranger_password = std::env::var("APP_SEED_STRANGER_PASSWORD");
    match (stranger_email, stranger_password) {
        (Ok(email), Ok(password)) => {
            if password.chars().count() < 8 {
                anyhow::bail!("APP_SEED_STRANGER_PASSWORD must be at least 8 characters");
            }
            let name =
                std::env::var("APP_SEED_STRANGER_NAME").unwrap_or_else(|_| "Stranger".to_string());
            // Create only — deliberately NOT added to any space.
            let _ = resolve_or_create_user(&repo, &email, &name, &password).await?;
        }
        (Ok(_), Err(_)) | (Err(_), Ok(_)) => {
            tracing::warn!(
                "APP_SEED_STRANGER_EMAIL and APP_SEED_STRANGER_PASSWORD must both be set; \
                 skipping stranger seed"
            );
        }
        (Err(_), Err(_)) if is_local => {
            // Local/dev default: stranger@test.com / stranger123456 (no space membership).
            let _ = resolve_or_create_user(
                &repo,
                "stranger@test.com",
                "Stranger",
                "stranger123456",
            )
            .await?;
        }
        (Err(_), Err(_)) => {
            tracing::debug!("No stranger seed configured; skipping (production, env unset)");
        }
    }

    Ok(())
}
