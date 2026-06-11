//! Axum route handlers for Apple Music endpoints.

use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::Query as QueryArray;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::apple_music::client::Artwork;
use crate::error::AppError;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// GET /apple/artist?name=<name>&name=<name>
// ---------------------------------------------------------------------------

/// Accepts one or more `name` query params.
/// Uses `axum_extra::extract::Query` to handle repeated keys.
#[derive(Deserialize)]
pub struct ArtistQuery {
    pub name: Vec<String>,
}

#[derive(Serialize)]
pub struct AppleArtistInfoResponse {
    pub name: String,
    pub id: String,
    pub apple_music_url: Option<String>,
    pub artwork: Option<Artwork>,
    pub genres: Vec<String>,
    pub similar_artists: Vec<SimilarArtist>,
}

#[derive(Serialize)]
pub struct SimilarArtist {
    pub id: String,
    pub name: String,
    pub apple_music_url: Option<String>,
    pub artwork: Option<Artwork>,
}

/// Look up one or more artists by name and return their info + similar artists.
///
/// Uses Developer Token auth (no user login required).
///
/// Apple Music endpoints used:
///   - `GET /v1/catalog/{storefront}/search` (types=artists)
///   - `GET /v1/catalog/{storefront}/artists/{id}/view/similar-artists`
pub async fn get_artist_info(
    State(state): State<AppState>,
    QueryArray(q): QueryArray<ArtistQuery>,
) -> Result<Json<Vec<AppleArtistInfoResponse>>, AppError> {
    if q.name.is_empty() {
        return Err(AppError::Internal(
            "At least one `name` query parameter is required".into(),
        ));
    }

    let am = state
        .apple_music_client
        .as_ref()
        .ok_or_else(|| AppError::Internal("Apple Music client not configured".into()))?;
    let token = state
        .apple_dev_token
        .as_ref()
        .ok_or_else(|| {
            AppError::Internal("Apple Music developer token manager not configured".into())
        })?
        .get_token()
        .await?;

    let mut results = Vec::with_capacity(q.name.len());

    for artist_name in &q.name {
        let search_results = am.search_artists(&token, artist_name, 1).await?;
        let artist = search_results
            .into_iter()
            .next()
            .ok_or_else(|| AppError::ArtistNotFound(artist_name.clone()))?;

        // Fetch similar artists via the view.
        let similar = am
            .get_similar_artists(&token, &artist.id)
            .await
            .unwrap_or_default();

        let attrs = artist.attributes.as_ref();

        results.push(AppleArtistInfoResponse {
            name: attrs.map(|a| a.name.clone()).unwrap_or_default(),
            id: artist.id,
            apple_music_url: attrs.and_then(|a| a.url.clone()),
            artwork: attrs.and_then(|a| a.artwork.clone()),
            genres: attrs.map(|a| a.genre_names.clone()).unwrap_or_default(),
            similar_artists: similar
                .into_iter()
                .map(|a| {
                    let a_attrs = a.attributes.as_ref();
                    SimilarArtist {
                        id: a.id,
                        name: a_attrs.map(|x| x.name.clone()).unwrap_or_default(),
                        apple_music_url: a_attrs.and_then(|x| x.url.clone()),
                        artwork: a_attrs.and_then(|x| x.artwork.clone()),
                    }
                })
                .collect(),
        });
    }

    Ok(Json(results))
}

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
