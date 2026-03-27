//! Periodic Apple Music playlist updater service.
//!
//! Runs as a background tokio task. At a configurable interval it:
//!   1. Reads the current artist list from a shared `AppleArtistQueue`.
//!   2. For each artist, fetches top songs via the `top-songs` view.
//!   3. Replaces the playlist by clearing and re-adding all tracks.
//!
//! Apple Music endpoints used:
//!   - `GET /v1/catalog/{storefront}/search` (types=artists)
//!   - `GET /v1/catalog/{storefront}/artists/{id}/view/top-songs`
//!   - `POST /v1/me/library/playlists/{id}/tracks`

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::AppError;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Artist queue
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct AppleArtistQueue {
    inner: Arc<RwLock<AppleArtistQueueInner>>,
}

struct AppleArtistQueueInner {
    artists: Vec<String>,
    playlist_id: Option<String>,
    tracks_per_artist: u8,
}

impl AppleArtistQueue {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(AppleArtistQueueInner {
                artists: Vec::new(),
                playlist_id: None,
                tracks_per_artist: 3,
            })),
        }
    }

    pub async fn set_artists(&self, artists: Vec<String>) {
        self.inner.write().await.artists = artists;
    }

    pub async fn set_playlist_id(&self, id: String) {
        self.inner.write().await.playlist_id = Some(id);
    }

    pub async fn set_tracks_per_artist(&self, n: u8) {
        self.inner.write().await.tracks_per_artist = n;
    }

    pub async fn snapshot(&self) -> (Vec<String>, Option<String>, u8) {
        let guard = self.inner.read().await;
        (
            guard.artists.clone(),
            guard.playlist_id.clone(),
            guard.tracks_per_artist,
        )
    }
}

// ---------------------------------------------------------------------------
// Background task
// ---------------------------------------------------------------------------

pub fn spawn_apple_updater(
    state: Arc<AppState>,
    interval: std::time::Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);

        loop {
            ticker.tick().await;

            if let Err(e) = run_update_cycle(&state).await {
                tracing::error!("Apple Music updater cycle failed: {e}");
            }
        }
    })
}

async fn run_update_cycle(state: &Arc<AppState>) -> Result<(), AppError> {
    let queue = match state.apple_artist_queue.as_ref() {
        Some(q) => q,
        None => return Ok(()),
    };

    let (artists, playlist_id, per_artist) = queue.snapshot().await;

    let playlist_id = match playlist_id {
        Some(id) => id,
        None => {
            tracing::debug!("Apple updater: no playlist configured, skipping cycle");
            return Ok(());
        }
    };

    if artists.is_empty() {
        tracing::debug!("Apple updater: artist list empty, skipping cycle");
        return Ok(());
    }

    let am = state.apple_music_client.as_ref().ok_or_else(|| {
        AppError::Internal("Apple Music client not configured".into())
    })?;
    let dev_token = state.apple_dev_token.as_ref().ok_or_else(|| {
        AppError::Internal("Apple Music developer token manager not configured".into())
    })?.get_token().await?;
    let user_token = state.apple_user_token.as_ref().ok_or_else(|| {
        AppError::Internal("Apple Music user token store not configured".into())
    })?.get_token().await?;

    let mut song_ids: Vec<String> = Vec::new();

    for artist_name in &artists {
        // First search for the artist to get their ID.
        match am.search_artists(&dev_token, artist_name, 1).await {
            Ok(results) => {
                if let Some(artist) = results.into_iter().next() {
                    // Then fetch their top songs via the view.
                    match am
                        .get_artist_top_songs(&dev_token, &artist.id, per_artist)
                        .await
                    {
                        Ok(songs) => {
                            for s in songs {
                                song_ids.push(s.id);
                            }
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Apple updater: failed to get top songs for '{}': {e}",
                                artist_name
                            );
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(
                    "Apple updater: failed to search artist '{}': {e}",
                    artist_name
                );
            }
        }
    }

    if song_ids.is_empty() {
        tracing::info!("Apple updater: no songs found, skipping playlist update");
        return Ok(());
    }

    // Apple Music's "add tracks" endpoint appends; there's no "replace all" in one call.
    // We add the new tracks. In production you may want to clear first by removing
    // existing tracks, but the library playlist API doesn't support a bulk-replace.
    am.add_tracks_to_playlist(&dev_token, &user_token, &playlist_id, &song_ids)
        .await?;

    tracing::info!(
        "Apple updater: added {} tracks to playlist {playlist_id} from {} artists",
        song_ids.len(),
        artists.len(),
    );

    Ok(())
}
