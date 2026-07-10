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
}
