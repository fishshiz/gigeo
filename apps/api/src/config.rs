use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub mapbox_private_key: String,
    pub ticketmaster_api_key: String,
}

impl AppConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        let mapbox_private_key =
            std::env::var("MAPBOX_PRIVATE_KEY").context("MAPBOX_PRIVATE_KEY must be set")?;

        let ticketmaster_api_key =
            std::env::var("TICKETMASTER_API_KEY").context("TICKETMASTER_API_KEY must be set")?;

        Ok(Self {
            mapbox_private_key,
            ticketmaster_api_key,
        })
    }
}
