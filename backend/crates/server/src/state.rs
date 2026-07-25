use std::sync::Arc;

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use argon2::Argon2;
use moka::future::Cache;
use sea_orm::DatabaseConnection;
use migration::MigratorTrait;
use shared_common::enums::UserRole;
use user_management::domain::user::entity::User;
use user_management::domain::user::repository::UserRepository;
use user_management::infrastructure::persistence::user_repo::SeaOrmUserRepo;

use crate::config::Configuration;

#[derive(Clone)]
pub struct AppState {
    pub db: DatabaseConnection,
    pub config: Arc<Configuration>,
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
        let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "local".to_string());
        let explicit_email = std::env::var("APP_SEED_ADMIN_EMAIL").ok();
        match explicit_email {
            Some(_) => {
                seed_admin(&db).await?;
            }
            None if app_env == "local" || app_env == "dev" => {
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

/// Idempotently ensure the test space exists (created by migration with a
/// fixed UUID) and, if an admin user is present, make that admin its owner.
/// Also seeds the E2E test users as space members (editor role) so that
/// integration/E2E tests can exercise the edit path instead of read-only mode.
async fn seed_test_space(db: &DatabaseConnection) -> anyhow::Result<()> {
    use sea_orm::ConnectionTrait;
    use uuid::Uuid;

    let test_space_id =
        Uuid::parse_str(migration::m20250101_000029_add_space_id_to_business_entities::TEST_SPACE_ID)
            .expect("TEST_SPACE_ID must be a valid UUID");

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
        let now = chrono::Utc::now();
        let insert = format!(
            r#"INSERT INTO "space_members" ("space_id","user_id","role","created_at","updated_at")
               VALUES ('{space}','{user}','owner','{now}','{now}')
               ON CONFLICT ("space_id","user_id") DO NOTHING"#,
            space = test_space_id,
            user = admin_id,
            now = now
        );
        let _ = db.execute_unprepared(&insert).await;
    }

    // Seed E2E test users and add them as space members (editor role) so that
    // tests can create/update/delete entities within the test space. These are
    // only seeded in local/dev environments to avoid leaking test accounts into
    // production.
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "local".to_string());
    if app_env == "local" || app_env == "dev" {
        let test_users = [
            ("test@example.com", "测试用户", "testpassword123"),
            ("e2e3@test.com", "E2E Test 3", "e2e123456"),
        ];
        let repo = SeaOrmUserRepo::new(db.clone());
        for (email, name, password) in test_users {
            let user_id = if let Some(existing) = repo.find_by_email(email).await? {
                existing.id
            } else {
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
                let saved = repo.save(&user).await?;
                saved.id
            };
            let now = chrono::Utc::now();
            let insert = format!(
                r#"INSERT INTO "space_members" ("space_id","user_id","role","created_at","updated_at")
                   VALUES ('{space}','{user}','editor','{now}','{now}')
                   ON CONFLICT ("space_id","user_id") DO NOTHING"#,
                space = test_space_id,
                user = user_id,
                now = now
            );
            let _ = db.execute_unprepared(&insert).await;
        }
    }
    Ok(())
}

async fn seed_admin(db: &DatabaseConnection) -> anyhow::Result<()> {
    let email = std::env::var("APP_SEED_ADMIN_EMAIL")
        .unwrap_or_else(|_| "admin@test.com".to_string());
    let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "local".to_string());
    let is_production = !(app_env == "local" || app_env == "dev");
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

    if password.len() < 8 {
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
