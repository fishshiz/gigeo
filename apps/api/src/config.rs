use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub mapbox_private_key: String,
    pub ticketmaster_api_key: String,
    pub cookie_domain: Option<String>,
    pub cookie_secure: bool,
    pub cookie_key: String,
    pub cors_allowed_origins: Vec<String>,
}

impl AppConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        let mapbox_private_key =
            std::env::var("MAPBOX_PRIVATE_KEY").context("MAPBOX_PRIVATE_KEY must be set")?;

        let ticketmaster_api_key =
            std::env::var("TICKETMASTER_API_KEY").context("TICKETMASTER_API_KEY must be set")?;

        let cookie_domain = std::env::var("COOKIE_DOMAIN").ok();
        let cookie_secure = std::env::var("COOKIE_SECURE")
            .map(|v| v == "true")
            .unwrap_or(false);
        let cookie_key = std::env::var("COOKIE_KEY").context("COOKIE_KEY must be set")?;

        let cors_allowed_origins = std::env::var("CORS_ALLOWED_ORIGINS")
            .unwrap_or_else(|_| "http://localhost:5173".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        Ok(Self {
            mapbox_private_key,
            ticketmaster_api_key,
            cookie_domain,
            cookie_secure,
            cookie_key,
            cors_allowed_origins,
        })
    }
}
