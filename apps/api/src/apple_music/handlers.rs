//! Axum route handlers for Apple Music endpoints.
//!
//! The on-demand `GET /apple/artist` lookup that used to live here was
//! retired in favor of persisted canonical-artist data (see
//! `docs/adr/0001-canonical-artist-model.md` and `crate::artists::lookup`)
//! — `EventDetails.tsx` now reads `performer.enrichment` directly off the
//! event payload instead of live-fetching per page view.

use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// POST /apple/playlist
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreateApplePlaylistRequest {
    /// Playlist name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Artist names to seed the playlist with.
    pub artists: Vec<String>,
    /// Number of songs per artist (default 3, max 10).
    pub tracks_per_artist: Option<u8>,
}

#[derive(Serialize)]
pub struct CreateApplePlaylistResponse {
    pub playlist_id: String,
    pub track_count: usize,
    pub tracks: Vec<ApplePlaylistTrack>,
}

#[derive(Serialize)]
pub struct ApplePlaylistTrack {
    pub name: String,
    pub artist: String,
    pub song_id: String,
}

/// Create an Apple Music library playlist from a list of artist names.
///
/// Requires a Music User Token (set via `POST /apple/user-token`).
///
/// Apple Music endpoints used:
///   - `GET /v1/catalog/{storefront}/search` (types=songs, searching by artist name)
///   - `POST /v1/me/library/playlists`
///   - `POST /v1/me/library/playlists/{id}/tracks`
pub async fn create_playlist(
    State(state): State<AppState>,
    Json(req): Json<CreateApplePlaylistRequest>,
) -> Result<(StatusCode, Json<CreateApplePlaylistResponse>), AppError> {
    let am = state
        .apple_music_client
        .as_ref()
        .ok_or_else(|| AppError::Internal("Apple Music client not configured".into()))?;
    let dev_token = state
        .apple_dev_token
        .as_ref()
        .ok_or_else(|| {
            AppError::Internal("Apple Music developer token manager not configured".into())
        })?
        .get_token()
        .await?;
    let user_token = state
        .apple_user_token
        .as_ref()
        .ok_or_else(|| AppError::Internal("Apple Music user token store not configured".into()))?
        .get_token()
        .await?;

    let per_artist = req.tracks_per_artist.unwrap_or(3).min(10);

    // Search for songs by each artist.
    let mut song_ids: Vec<String> = Vec::new();
    let mut playlist_tracks: Vec<ApplePlaylistTrack> = Vec::new();

    for artist_name in &req.artists {
        let songs = am.search_songs(&dev_token, artist_name, per_artist).await?;

        for song in songs {
            let attrs = song.attributes.as_ref();
            playlist_tracks.push(ApplePlaylistTrack {
                name: attrs.map(|a| a.name.clone()).unwrap_or_default(),
                artist: attrs.map(|a| a.artist_name.clone()).unwrap_or_default(),
                song_id: song.id.clone(),
            });
            song_ids.push(song.id);
        }
    }

    // Create the playlist.
    let playlist = am
        .create_library_playlist(
            &dev_token,
            &user_token,
            &req.name,
            req.description.as_deref(),
        )
        .await?;

    // Add tracks to the playlist.
    if !song_ids.is_empty() {
        am.add_tracks_to_playlist(&dev_token, &user_token, &playlist.id, &song_ids)
            .await?;
    }

    Ok((
        StatusCode::CREATED,
        Json(CreateApplePlaylistResponse {
            playlist_id: playlist.id,
            track_count: playlist_tracks.len(),
            tracks: playlist_tracks,
        }),
    ))
}

// ---------------------------------------------------------------------------
// POST /apple/user-token  — store a Music User Token from the frontend
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct SetUserTokenRequest {
    pub music_user_token: String,
}

/// Store a Music User Token obtained from MusicKit JS on the frontend.
/// This token is required for user-scoped endpoints (library playlists).
pub async fn set_user_token(
    State(state): State<AppState>,
    Json(req): Json<SetUserTokenRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let store = state
        .apple_user_token
        .as_ref()
        .ok_or_else(|| AppError::Internal("Apple Music user token store not configured".into()))?;

    store.set_token(req.music_user_token).await;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "Music User Token stored. You can now create playlists."
    })))
}
