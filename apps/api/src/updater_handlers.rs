//! HTTP handlers for controlling the periodic playlist updater.

use axum::{extract::State, http::StatusCode, Json};
use serde::Deserialize;
use std::sync::Arc;

use crate::error::AppError;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// POST /updater/config
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct UpdaterConfigRequest {
    /// The Spotify playlist ID to keep updated.
    pub playlist_id: String,
    /// Artist names to seed the playlist with.
    pub artists: Vec<String>,
    /// Number of tracks per artist (default 3, max 10).
    pub tracks_per_artist: Option<u8>,
}

#[derive(serde::Serialize)]
pub struct UpdaterConfigResponse {
    pub status: String,
    pub playlist_id: String,
    pub artist_count: usize,
    pub tracks_per_artist: u8,
}

/// Configure the periodic updater with a playlist ID and artist list.
///
/// The background task will pick up these values on its next cycle
/// and replace the playlist contents with tracks from the given artists.
pub async fn configure_updater(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdaterConfigRequest>,
) -> Result<(StatusCode, Json<UpdaterConfigResponse>), AppError> {
    let per_artist = req.tracks_per_artist.unwrap_or(3).min(10);

    state.artist_queue.set_playlist_id(req.playlist_id.clone()).await;
    state.artist_queue.set_artists(req.artists.clone()).await;
    state.artist_queue.set_tracks_per_artist(per_artist).await;

    Ok((
        StatusCode::OK,
        Json(UpdaterConfigResponse {
            status: "configured".into(),
            playlist_id: req.playlist_id,
            artist_count: req.artists.len(),
            tracks_per_artist: per_artist,
        }),
    ))
}

// ---------------------------------------------------------------------------
// PUT /updater/artists
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct UpdateArtistsRequest {
    pub artists: Vec<String>,
}

/// Replace the updater's artist list without changing the target playlist.
/// The next updater cycle will use this new list.
pub async fn update_artists(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpdateArtistsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.artist_queue.set_artists(req.artists.clone()).await;

    Ok(Json(serde_json::json!({
        "status": "updated",
        "artist_count": req.artists.len(),
    })))
}
