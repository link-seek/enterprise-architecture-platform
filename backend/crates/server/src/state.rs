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
        let db = sea_orm::Database::connect(&config.database.url).await?;

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
