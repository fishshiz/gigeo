//! HTTP handlers for controlling the Apple Music periodic playlist updater.

use axum::{Json, extract::State, http::StatusCode};
use serde::Deserialize;

use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct AppleUpdaterConfigRequest {
    /// The Apple Music library playlist ID to keep updated.
    pub playlist_id: String,
    /// Artist names to seed the playlist with.
    pub artists: Vec<String>,
    /// Number of tracks per artist (default 3, max 10).
    pub tracks_per_artist: Option<u8>,
}

#[derive(serde::Serialize)]
pub struct AppleUpdaterConfigResponse {
    pub status: String,
    pub playlist_id: String,
    pub artist_count: usize,
    pub tracks_per_artist: u8,
}

/// Configure the Apple Music periodic updater.
pub async fn configure_apple_updater(
    State(state): State<AppState>,
    Json(req): Json<AppleUpdaterConfigRequest>,
) -> Result<(StatusCode, Json<AppleUpdaterConfigResponse>), AppError> {
    let queue = state
        .apple_artist_queue
        .as_ref()
        .ok_or_else(|| AppError::Internal("Apple Music updater not configured".into()))?;

    let per_artist = req.tracks_per_artist.unwrap_or(3).min(10);

    queue.set_playlist_id(req.playlist_id.clone()).await;
    queue.set_artists(req.artists.clone()).await;
    queue.set_tracks_per_artist(per_artist).await;

    Ok((
        StatusCode::OK,
        Json(AppleUpdaterConfigResponse {
            status: "configured".into(),
            playlist_id: req.playlist_id,
            artist_count: req.artists.len(),
            tracks_per_artist: per_artist,
        }),
    ))
}

#[derive(Deserialize)]
pub struct AppleUpdateArtistsRequest {
    pub artists: Vec<String>,
}

/// Replace the Apple Music updater's artist list.
pub async fn update_apple_artists(
    State(state): State<AppState>,
    Json(req): Json<AppleUpdateArtistsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let queue = state
        .apple_artist_queue
        .as_ref()
        .ok_or_else(|| AppError::Internal("Apple Music updater not configured".into()))?;

    queue.set_artists(req.artists.clone()).await;

    Ok(Json(serde_json::json!({
        "status": "updated",
        "artist_count": req.artists.len(),
    })))
}
