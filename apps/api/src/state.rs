use reqwest::Client;

use crate::auth::{ClientCredentialsManager, UserTokenManager};
use crate::db::AppDatabase;
use crate::spotify::client::SpotifyClient;

use crate::apple_music::artwork_cache::ArtworkCache;
use crate::apple_music::auth::DeveloperTokenManager;
use crate::apple_music::client::AppleMusicClient;
use axum_extra::extract::cookie::Key;
use std::{ops::Deref, sync::Arc};

use axum::extract::FromRef;

#[derive(Clone)]
pub struct AppState(pub Arc<AppStateInner>);

pub struct AppStateInner {
    pub db: AppDatabase,
    pub client: Client,
    pub mapbox_key: String,
    pub ticketmaster_key: String,
    pub spotify: SpotifyClient,
    pub cc_manager: Arc<ClientCredentialsManager>,
    pub user_manager: Arc<UserTokenManager>,

    pub cookie_secure: bool,
    pub cookie_domain: Option<String>,
    pub cookie_key: Key,

    pub spotify_client_id: String,
    pub spotify_client_secret: String,
    pub apple_music_client: Option<AppleMusicClient>,
    pub apple_dev_token: Option<Arc<DeveloperTokenManager>>,
    /// Shared regardless of whether Apple Music is configured — cheap to
    /// hold, and only ever consulted when `apple_music_client`/
    /// `apple_dev_token` are also present (see
    /// `crate::predicthq::backfill_artwork`).
    pub apple_artwork_cache: Arc<ArtworkCache>,

    /// `None` when `PREDICTHQ_API_KEY` isn't set — PredictHQ isn't wired
    /// into any live route yet (see `crate::predicthq`), so this is
    /// deliberately optional rather than a boot-time requirement.
    pub predicthq_api_key: Option<String>,
}

impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
