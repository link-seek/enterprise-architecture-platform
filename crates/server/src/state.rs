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

        // Seed admin test account (only in non-production environments)
        let app_env = std::env::var("APP_ENV").unwrap_or_else(|_| "local".to_string());
        if app_env == "local" || app_env == "dev" {
            seed_admin(&db).await?;
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
    let email = std::env::var("APP_SEED_ADMIN_EMAIL").unwrap_or_else(|_| "admin@test.com".to_string());
    let password = std::env::var("APP_SEED_ADMIN_PASSWORD").unwrap_or_else(|_| "admin123456".to_string());
    let name = std::env::var("APP_SEED_ADMIN_NAME").unwrap_or_else(|_| "Admin".to_string());

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
    }
    Ok(())
}
