use super::db::{CreatePlaylistParams, create_playlist_record, resolve_session_account};
use crate::error::AppError;
use crate::services::playlist_builder::{
    PlaylistVisibility, find_artist_names_near, resolve_update_mode, search_tracks_for_artists,
};
use crate::state::AppState;
use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::cookie::SignedCookieJar;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// POST /playlist
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreatePlaylistRequest {
    /// Human-readable playlist name.
    pub name: String,
    /// Optional description.
    pub description: Option<String>,
    /// Number of tracks to include per artist (default 3, max 10).
    pub tracks_per_artist: Option<u8>,
    // How often to update playlist, in days (min 1, max 60).
    pub cadence: Option<u8>,
    // Whether playlist is public or private.
    pub privacy: bool,
    // Whether updates replace the playlist's tracks (true) or add to them
    // (false/omitted). Defaults to true.
    pub destructive: Option<bool>,
    // The centered location of the playlist.
    pub location: String,
    // The location coordinates
    pub latitude: f64,
    pub longitude: f64,
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

// ---------------------------------------------------------------------------
// GET /playlist
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct PlaylistSummary {
    pub id: String,
    pub name: String,
    pub external_url: Option<String>,
    pub images: Vec<crate::spotify::client::Image>,
    pub track_count: u32,
    pub city: String,
}

pub async fn get_user_playlists(
    State(state): State<AppState>,
    jar: SignedCookieJar,
) -> Result<Json<Vec<PlaylistSummary>>, AppError> {
    let cookie = jar
        .get("spotify_oauth_state")
        .ok_or(AppError::Unauthorized {
            status: StatusCode::UNAUTHORIZED,
            message: "Unauthorized".into(),
        })?;
    let session_id = uuid::Uuid::parse_str(cookie.value()).map_err(|_| AppError::Unauthorized {
        status: StatusCode::UNAUTHORIZED,
        message: "Unauthorized session id".into(),
    })?;

    let Some(account_id) = resolve_session_account(&state.db.pool, session_id).await? else {
        return Err(AppError::Unauthorized {
            status: StatusCode::UNAUTHORIZED,
            message: "Unauthorized user id".into(),
        });
    };
    let playlists = get_playlists(&state, account_id).await?;
    Ok(playlists)
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
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Json(req): Json<CreatePlaylistRequest>,
) -> Result<(StatusCode, Json<CreatePlaylistResponse>), AppError> {
    let cookie = jar
        .get("spotify_oauth_state")
        .ok_or_else(|| AppError::AuthRequired("No session cookie. Visit /login first.".into()))?;
    let session_id = uuid::Uuid::parse_str(cookie.value()).map_err(|_| AppError::Unauthorized {
        status: StatusCode::UNAUTHORIZED,
        message: "Invalid session cookie".into(),
    })?;
    let account_id = resolve_session_account(&state.db.pool, session_id)
        .await?
        .ok_or_else(|| {
            AppError::AuthRequired("Session expired or invalid. Visit /login first.".into())
        })?;

    let user_token = crate::spotify::token::get_valid_spotify_token(&state, account_id).await?;
    let cc_token = state.cc_manager.get_token().await?;

    let per_artist = req.tracks_per_artist.unwrap_or(3).min(10);

    let artist_names = find_artist_names_near(&state, req.latitude, req.longitude, 25, 7).await?;

    if artist_names.is_empty() {
        return Err(AppError::Internal(
            "No artists found from nearby Ticketmaster events".to_string(),
        ));
    }

    let track_results =
        search_tracks_for_artists(&state, &cc_token.access_token, &artist_names, per_artist)
            .await?;

    if track_results.is_empty() {
        return Err(AppError::ArtistNotFound(
            "No Spotify tracks found for artists in nearby events".to_string(),
        ));
    }

    let track_uris: Vec<String> = track_results.iter().map(|t| t.uri.clone()).collect();
    let playlist_tracks: Vec<PlaylistTrack> = track_results
        .into_iter()
        .map(|t| PlaylistTrack {
            name: t.name,
            artist: t.artist,
            uri: t.uri,
        })
        .collect();

    let playlist = state
        .spotify
        .create_playlist(
            &user_token,
            &req.name,
            req.description.as_deref(),
            !req.privacy,
        )
        .await?;

    for chunk in track_uris.chunks(100) {
        state
            .spotify
            .add_items_to_playlist(&user_token, &playlist.id, chunk)
            .await?;
    }

    let geohash_str = geohash::encode(
        geohash::Coord {
            x: req.longitude,
            y: req.latitude,
        },
        6,
    )
    .map_err(|e| AppError::Internal(format!("geohash encode failed: {e}")))?;

    let cadence_days: i16 = match req.cadence {
        Some(7) => 7,
        Some(30) => 30,
        Some(60) => 60,
        Some(other) if other <= 7 => 7,
        Some(other) if other <= 30 => 30,
        Some(_) => 60,
        None => 30,
    };

    let visibility = if req.privacy {
        PlaylistVisibility::Private
    } else {
        PlaylistVisibility::Public
    };

    let update_mode = resolve_update_mode(req.destructive);

    create_playlist_record(
        &state.db.pool,
        CreatePlaylistParams {
            account_id,
            provider_playlist_id: &playlist.id,
            name: &req.name,
            geohash: &geohash_str,
            city: &req.location,
            update_cadence_days: cadence_days,
            visibility,
            update_mode,
        },
    )
    .await?;

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

// GET /playlist
pub async fn get_playlists(
    state: &AppState,
    account_id: uuid::Uuid,
) -> Result<Json<Vec<PlaylistSummary>>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT provider_playlist_id, city
        FROM playlist
        WHERE account_id = $1
        "#,
        account_id
    )
    .fetch_all(&state.db.pool)
    .await?;

    if rows.is_empty() {
        return Ok(Json(vec![]));
    }

    let token = crate::spotify::token::get_valid_spotify_token(state, account_id).await?;

    let mut summaries = Vec::with_capacity(rows.len());
    for row in rows {
        match state
            .spotify
            .get_playlist(&token, &row.provider_playlist_id)
            .await
        {
            Ok(details) => summaries.push(PlaylistSummary {
                id: details.id,
                name: details.name,
                external_url: details.external_urls.spotify,
                images: details.images,
                track_count: details.tracks.total,
                city: row.city,
            }),
            Err(err) => {
                // A single playlist that's been deleted/renamed on Spotify's side
                // (or a transient API error) shouldn't take down the whole list.
                tracing::warn!(
                    playlist_id = %row.provider_playlist_id,
                    error = %err,
                    "failed to fetch playlist details from Spotify, skipping"
                );
            }
        }
    }

    Ok(Json(summaries))
}
