use reqwest::Client;
use std::sync::Arc;

use crate::auth::{ClientCredentialsManager, UserTokenManager};
use crate::spotify::SpotifyClient;
use crate::updater::ArtistQueue;

use crate::apple_music::auth::{DeveloperTokenManager, MusicUserTokenStore};
use crate::apple_music::client::AppleMusicClient;
use crate::apple_music::updater::AppleArtistQueue;

pub struct AppState {
    pub client: Client,
    pub mapbox_key: String,
    pub ticketmaster_key: String,
    pub spotify: SpotifyClient,
    pub cc_manager: Arc<ClientCredentialsManager>,
    pub user_manager: Arc<UserTokenManager>,
    pub artist_queue: ArtistQueue,

    // -- Apple Music --
    pub apple_music_client: Option<AppleMusicClient>,
    pub apple_dev_token: Option<Arc<DeveloperTokenManager>>,
    pub apple_user_token: Option<Arc<MusicUserTokenStore>>,
    pub apple_artist_queue: Option<AppleArtistQueue>,
}
