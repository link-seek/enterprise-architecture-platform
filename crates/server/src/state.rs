use std::sync::Arc;

use moka::future::Cache;
use sea_orm::DatabaseConnection;
use migration::MigratorTrait;

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
