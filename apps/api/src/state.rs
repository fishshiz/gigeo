use reqwest::Client;

use crate::auth::{ClientCredentialsManager, UserTokenManager};
use crate::db::AppDatabase;
use crate::spotify::client::SpotifyClient;

use crate::apple_music::auth::{DeveloperTokenManager, MusicUserTokenStore};
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
    pub apple_user_token: Option<Arc<MusicUserTokenStore>>,
}

impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
