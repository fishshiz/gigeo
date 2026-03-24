use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub scrape_interval_hours: u64,
    pub scrape_window_days: u32,
    pub mapbox_private_key: String,
    pub ticketmaster_api_key: String,
}

/// City definition for seeding the database.
#[derive(Debug, Clone)]
pub struct CityDef {
    pub name: &'static str,
    pub country_code: &'static str,
    pub state: &'static str,
    pub id: &'static str,
}

/// Default cities to scrape.
pub const DEFAULT_CITIES: &[CityDef] = &[
    CityDef {
        name: "Boston",
        country_code: "US",
        state: "MA",
        id: "dXJuOm1ieHBsYzpBZ1ZvN0E",
    },
    CityDef {
        name: "New York",
        country_code: "US",
        state: "NY",
        id: "dXJuOm1ieHBsYzpEZTVJN0E",
    },
];

impl AppConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;

        let scrape_interval_hours = std::env::var("SCRAPE_INTERVAL_HOURS")
            .unwrap_or_else(|_| "6".to_string())
            .parse::<u64>()
            .context("SCRAPE_INTERVAL_HOURS must be a valid integer")?;

        let scrape_window_days = std::env::var("SCRAPE_WINDOW_DAYS")
            .unwrap_or_else(|_| "30".to_string())
            .parse::<u32>()
            .context("SCRAPE_WINDOW_DAYS must be a valid integer")?;

        let mapbox_private_key =
            std::env::var("MAPBOX_PRIVATE_KEY").context("MAPBOX_PRIVATE_KEY must be set")?;

        let ticketmaster_api_key =
            std::env::var("TICKETMASTER_API_KEY").context("TICKETMASTER_API_KEY must be set")?;

        Ok(Self {
            database_url,
            scrape_interval_hours,
            scrape_window_days,
            mapbox_private_key,
            ticketmaster_api_key,
        })
    }
}
