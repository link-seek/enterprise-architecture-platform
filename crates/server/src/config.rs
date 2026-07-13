use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
    pub oauth: OAuthConfig,
    pub llm: LlmConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub access_token_ttl_minutes: u64,
    pub refresh_token_ttl_days: u64,
    pub rsa_private_key_pem: String,
    pub rsa_public_key_pem: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthConfig {
    pub clients: Vec<OAuthClientConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthClientConfig {
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uris: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub backend: String,
    pub api_key: Option<String>,
    pub model: Option<String>,
    pub endpoint: Option<String>,
}

impl Configuration {
    pub fn load() -> anyhow::Result<Self> {
        let mut builder = config::Config::builder()
            .add_source(config::File::with_name("config/default").required(false));

        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "local".to_string());
        builder = builder.add_source(
            config::File::with_name(&format!("config/{env}")).required(false),
        );

        builder = builder
            .add_source(config::Environment::with_prefix("APP").separator("__"));

        let cfg: Self = builder.build()?.try_deserialize()?;
        Ok(cfg)
    }

    /// Fill in empty config values with sensible defaults for development.
    ///
    /// - JWT secret: generate random 32-byte hex if empty
    /// - SQLite data/ directory: create if it doesn't exist
    /// - LLM api_key: warn if empty (degraded mode, non-blocking)
    pub fn ensure_defaults(&mut self) -> anyhow::Result<()> {
        if self.jwt.rsa_private_key_pem.is_empty() {
            let random_bytes: [u8; 32] = rand::random();
            let hex_secret: String = random_bytes.iter().map(|b| format!("{b:02x}")).collect();
            tracing::warn!(
                "JWT secret (rsa_private_key_pem) is empty — generated random secret for development. \
                 Set a fixed value in production via environment variable APP_JWT__RSA_PRIVATE_KEY_PEM."
            );
            self.jwt.rsa_private_key_pem = hex_secret;
        }

        if self.jwt.rsa_public_key_pem.is_empty() {
            self.jwt.rsa_public_key_pem = self.jwt.rsa_private_key_pem.clone();
        }

        if let Some(data_dir) = self.extract_sqlite_data_dir() {
            if !data_dir.exists() {
                tracing::warn!("SQLite data directory {:?} does not exist — creating it.", data_dir);
                std::fs::create_dir_all(&data_dir)?;
            }
        }

        if self.llm.api_key.as_ref().is_none_or(|k| k.is_empty()) {
            tracing::warn!(
                "LLM api_key is empty — AI features will be degraded. \
                 Set APP_LLM__API_KEY to enable."
            );
        }

        Ok(())
    }

    /// Extract the directory path from a `sqlite://./data/foo.db?mode=rwc` URL.
    fn extract_sqlite_data_dir(&self) -> Option<std::path::PathBuf> {
        let url = &self.database.url;
        let path_part = url.strip_prefix("sqlite://")?;
        let path_part = path_part.split('?').next()?;
        let path = std::path::Path::new(path_part);
        path.parent().filter(|p| !p.as_os_str().is_empty()).map(std::path::PathBuf::from)
    }
}
