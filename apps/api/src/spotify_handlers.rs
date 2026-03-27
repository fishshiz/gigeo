use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    Json,
};
use axum_extra::extract::Query as QueryArray;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::error::AppError;
use crate::spotify::Image;
use crate::state::AppState;

// ---------------------------------------------------------------------------
// GET /artist?name=<artist_name>&name=<artist_name>&...
// ---------------------------------------------------------------------------

/// Accepts one or more `name` query params:
///   GET /artist?name=Diana+Krall
///   GET /artist?name=Radiohead&name=Bjork&name=Aphex+Twin
///
/// Uses `axum_extra::extract::Query` which handles repeated keys
/// natively (backed by `serde_html_form` instead of `serde_urlencoded`).
#[derive(Deserialize)]
pub struct ArtistQuery {
    pub name: Vec<String>,
}

#[derive(Serialize)]
pub struct ArtistInfoResponse {
    pub name: String,
    pub id: String,
    pub uri: String,
    pub spotify_url: Option<String>,
    pub images: Vec<Image>,
}

/// Look up one or more artists by name and return their info
///
/// Uses Client Credentials auth (no user login required).
///
/// Spotify endpoints used:
///   - `GET /search` (type=artist) — non-deprecated
pub async fn get_artist_info(
    State(state): State<Arc<AppState>>,
    QueryArray(q): QueryArray<ArtistQuery>,
) -> Result<Json<Vec<ArtistInfoResponse>>, AppError> {
    if q.name.is_empty() {
        return Err(AppError::Internal(
            "At least one `name` query parameter is required".into(),
        ));
    }

    let token = state.cc_manager.get_token().await?;
    let mut results = Vec::with_capacity(q.name.len());

    for artist_name in &q.name {
        // Search for the artist by name.
        let search_results = state.spotify.search_artist(&token, artist_name, 1).await?;
        let artist = search_results
            .into_iter()
            .next()
            .ok_or_else(|| AppError::ArtistNotFound(artist_name.clone()))?;

        results.push(ArtistInfoResponse {
            name: artist.name,
            id: artist.id,
            uri: artist.uri,
            spotify_url: artist.external_urls.spotify,
            images: artist.images,
        });
    }

    Ok(Json(results))
}

// ---------------------------------------------------------------------------
// POST /playlist
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreatePlaylistRequest {
    /// Human-readable playlist name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// List of artist names to seed the playlist with.
    pub artists: Vec<String>,
    /// Number of tracks to include per artist (default 3, max 10).
    pub tracks_per_artist: Option<u8>,
}

#[derive(Serialize)]
pub struct CreatePlaylistResponse {
    pub playlist_id: String,
    pub playlist_url: Option<String>,
    pub track_count: usize,
    pub tracks: Vec<PlaylistTrack>,
}

#[derive(Serialize)]
pub struct PlaylistTrack {
    pub name: String,
    pub artist: String,
    pub uri: String,
}

/// Create a Spotify playlist from a list of artist names.
///
/// Requires the user to have completed the Authorization Code flow
/// (visit `/login` first).
///
/// Spotify endpoints used:
///   - `GET /search` (type=track, artist filter) — non-deprecated
///   - `POST /me/playlists` — requires `playlist-modify-public` / `playlist-modify-private`
///   - `POST /playlists/{id}/items` — requires same scopes
pub async fn create_playlist(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePlaylistRequest>,
) -> Result<(StatusCode, Json<CreatePlaylistResponse>), AppError> {
    let user_token = state.user_manager.get_token().await?;
    let cc_token = state.cc_manager.get_token().await?;

    let per_artist = req.tracks_per_artist.unwrap_or(3).min(10);

    // Gather tracks for each artist via search (non-deprecated).
    let mut track_uris: Vec<String> = Vec::new();
    let mut playlist_tracks: Vec<PlaylistTrack> = Vec::new();

    for artist_name in &req.artists {
        let tracks = state
            .spotify
            .search_tracks_by_artist(&cc_token, artist_name, per_artist)
            .await?;

        for t in tracks {
            playlist_tracks.push(PlaylistTrack {
                name: t.name,
                artist: t
                    .artists
                    .first()
                    .map(|a| a.name.clone())
                    .unwrap_or_default(),
                uri: t.uri.clone(),
            });
            track_uris.push(t.uri);
        }
    }

    // Create the playlist.
    let playlist = state
        .spotify
        .create_playlist(&user_token, &req.name, req.description.as_deref(), true)
        .await?;

    // Add tracks in batches of 100 (Spotify limit per request).
    for chunk in track_uris.chunks(100) {
        state
            .spotify
            .add_items_to_playlist(&user_token, &playlist.id, chunk)
            .await?;
    }

    Ok((
        StatusCode::CREATED,
        Json(CreatePlaylistResponse {
            playlist_id: playlist.id,
            playlist_url: playlist.external_urls.spotify,
            track_count: playlist_tracks.len(),
            tracks: playlist_tracks,
        }),
    ))
}

// ---------------------------------------------------------------------------
// OAuth callback handlers
// ---------------------------------------------------------------------------

/// `GET /login` — redirects the user to Spotify's authorization page.
pub async fn login(State(state): State<Arc<AppState>>) -> Response {
    let state_param = generate_state();
    let url = state.user_manager.authorize_url(&state_param);
    Redirect::temporary(&url).into_response()
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub error: Option<String>,
    pub state: Option<String>,
}

/// `GET /callback` — Spotify redirects here after user authorization.
/// Exchanges the authorization code for access + refresh tokens.
pub async fn oauth_callback(
    State(state): State<Arc<AppState>>,
    Query(q): Query<CallbackQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(err) = q.error {
        return Err(AppError::AuthRequired(format!(
            "User denied authorization: {err}"
        )));
    }

    let code = q
        .code
        .ok_or_else(|| AppError::AuthRequired("Missing authorization code".into()))?;

    state.user_manager.exchange_code(&code).await?;

    Ok(Json(serde_json::json!({
        "status": "ok",
        "message": "Authorization successful. You can now create playlists."
    })))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn generate_state() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..16)
        .map(|_| format!("{:02x}", rng.gen::<u8>()))
        .collect()
}
