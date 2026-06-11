//! Periodic playlist updater service.
//!
//! Runs as a background tokio task. At a configurable interval it:
//!   1. Reads the current artist list from a shared `ArtistQueue`.
//!   2. Searches for tracks per artist via `GET /search` (non-deprecated).
//!   3. Replaces the playlist contents via `PUT /playlists/{id}/items`.
//!
//! The artist queue can be updated at any time through the
//! `POST /updater/artists` endpoint.

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::error::AppError;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// Artist queue — thread-safe list of artist names to rotate through
// ---------------------------------------------------------------------------

/// A shared, mutable list of artist names that the updater reads from.
/// The HTTP handler can push new names or replace the whole list.
#[derive(Clone)]
pub struct ArtistQueue {
    inner: Arc<RwLock<ArtistQueueInner>>,
}

struct ArtistQueueInner {
    artists: Vec<String>,
    /// Spotify playlist ID to keep updated.
    playlist_id: Option<String>,
    /// Tracks per artist to include.
    tracks_per_artist: u8,
}

impl ArtistQueue {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(ArtistQueueInner {
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

/// Spawn a tokio task that periodically updates the playlist.
///
/// `interval` controls how often the updater runs.
pub fn spawn_updater(
    state: Arc<AppState>,
    interval: std::time::Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);

        loop {
            ticker.tick().await;

            if let Err(e) = run_update_cycle(&state).await {
                tracing::error!("Updater cycle failed: {e}");
            }
        }
    })
}

async fn run_update_cycle(state: &Arc<AppState>) -> Result<(), AppError> {
    let (artists, playlist_id, per_artist) = state.artist_queue.snapshot().await;

    let playlist_id = match playlist_id {
        Some(id) => id,
        None => {
            tracing::debug!("Updater: no playlist configured, skipping cycle");
            return Ok(());
        }
    };

    if artists.is_empty() {
        tracing::debug!("Updater: artist list empty, skipping cycle");
        return Ok(());
    }

    // Need user token for playlist mutation.
    let user_token = state.user_manager.get_token().await?;
    // Client credentials token for search.
    let cc_token = state.cc_manager.get_token().await?;

    let mut uris: Vec<String> = Vec::new();

    for artist_name in &artists {
        let tracks = state
            .spotify
            .search_tracks_by_artist(&cc_token.access_token, artist_name, per_artist)
            .await?;

        if let Some(tracks) = tracks {
            for t in tracks {
                uris.push(t.uri);
            }
        } else {
            tracing::warn!("Updater: failed to search tracks for {artist_name}");
        }
    }

    if uris.is_empty() {
        tracing::info!("Updater: no tracks found, skipping playlist replace");
        return Ok(());
    }

    // Truncate to 100 to respect the single-request limit.
    // For larger sets you would batch with add_items after clearing.
    uris.truncate(100);

    state
        .spotify
        .replace_playlist_items(&user_token, &playlist_id, &uris)
        .await?;

    tracing::info!(
        "Updater: replaced playlist {playlist_id} with {} tracks from {} artists",
        uris.len(),
        artists.len(),
    );

    Ok(())
}
