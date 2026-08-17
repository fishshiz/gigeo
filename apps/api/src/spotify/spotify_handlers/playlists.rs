use super::db::resolve_account_from_cookie;
use crate::accounts::db::{
    CreatePlaylistParams, UpdatePlaylistParams, create_playlist_record, deactivate_playlist,
    get_playlist_for_account, soft_delete_playlist, update_playlist_config,
};
use crate::error::AppError;
use crate::services::playlist_builder::{
    PlaylistUpdateMode, PlaylistVisibility, find_artist_names_near, resolve_update_mode,
    search_tracks_for_artists, validate_cadence_days,
};
use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::SignedCookieJar;
use serde::{Deserialize, Serialize};

/// Returns `true` if a Spotify API error indicates the resource no longer
/// exists upstream (as opposed to a transient failure).
fn is_gone_from_spotify(err: &AppError) -> bool {
    matches!(err, AppError::SpotifyApi { status, .. } if status.as_u16() == 404)
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
    pub playlist_id: String,
    pub id: String,
    pub name: String,
    pub external_url: Option<String>,
    pub images: Vec<crate::spotify::client::Image>,
    pub track_count: u32,
    pub city: String,
    pub visibility: PlaylistVisibility,
    pub update_mode: PlaylistUpdateMode,
    pub update_cadence_days: i16,
    pub is_active: bool,
}

pub async fn get_user_playlists(
    State(state): State<AppState>,
    jar: SignedCookieJar,
) -> Result<Json<Vec<PlaylistSummary>>, AppError> {
    let account_id = resolve_account_from_cookie(&state, &jar).await?;
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
    let account_id = resolve_account_from_cookie(&state, &jar).await?;

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

    metrics::counter!("playlist_created", "service" => "spotify").increment(1);

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
        SELECT
            id, provider_playlist_id, city,
            visibility as "visibility: PlaylistVisibility",
            update_mode as "update_mode: PlaylistUpdateMode",
            update_cadence_days, is_active
        FROM playlist
        WHERE account_id = $1 AND deleted_at IS NULL
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
        if !row.is_active {
            // Already known to be gone from Spotify — no point re-fetching.
            summaries.push(PlaylistSummary {
                playlist_id: row.id.to_string(),
                id: row.provider_playlist_id,
                name: String::new(),
                external_url: None,
                images: vec![],
                track_count: 0,
                city: row.city,
                visibility: row.visibility,
                update_mode: row.update_mode,
                update_cadence_days: row.update_cadence_days,
                is_active: false,
            });
            continue;
        }

        match state
            .spotify
            .get_playlist(&token, &row.provider_playlist_id)
            .await
        {
            Ok(details) => summaries.push(PlaylistSummary {
                playlist_id: row.id.to_string(),
                id: details.id,
                name: details.name,
                external_url: details.external_urls.spotify,
                images: details.images,
                track_count: details.tracks.total,
                city: row.city,
                visibility: row.visibility,
                update_mode: row.update_mode,
                update_cadence_days: row.update_cadence_days,
                is_active: true,
            }),
            Err(err) if is_gone_from_spotify(&err) => {
                if let Err(e) = deactivate_playlist(&state.db.pool, row.id).await {
                    tracing::warn!(playlist_id = %row.id, error = %e, "failed to persist is_active=false");
                }
                summaries.push(PlaylistSummary {
                    playlist_id: row.id.to_string(),
                    id: row.provider_playlist_id,
                    name: String::new(),
                    external_url: None,
                    images: vec![],
                    track_count: 0,
                    city: row.city,
                    visibility: row.visibility,
                    update_mode: row.update_mode,
                    update_cadence_days: row.update_cadence_days,
                    is_active: false,
                });
            }
            Err(err) => {
                // A transient API error shouldn't take down the whole list,
                // and shouldn't flip is_active (it may well still exist).
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

// ---------------------------------------------------------------------------
// PATCH /playlist/{id}
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct UpdatePlaylistRequest {
    pub name: String,
    pub privacy: bool,
    pub cadence: i16,
    pub destructive: bool,
}

#[derive(Serialize)]
pub struct UpdatePlaylistResponse {
    pub playlist_id: String,
    pub name: String,
    pub visibility: PlaylistVisibility,
    pub update_mode: PlaylistUpdateMode,
    pub update_cadence_days: i16,
}

pub async fn update_playlist(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Path(playlist_id): Path<uuid::Uuid>,
    Json(req): Json<UpdatePlaylistRequest>,
) -> Result<Json<UpdatePlaylistResponse>, AppError> {
    let account_id = resolve_account_from_cookie(&state, &jar).await?;
    let cadence = validate_cadence_days(req.cadence)?;

    let row = get_playlist_for_account(&state.db.pool, playlist_id, account_id)
        .await?
        .ok_or(AppError::PlaylistNotFound)?;

    if !row.is_active {
        // Nothing on Spotify's side left to apply an edit to.
        return Err(AppError::PlaylistNotFound);
    }

    let token = crate::spotify::token::get_valid_spotify_token(&state, account_id).await?;

    let visibility = if req.privacy {
        PlaylistVisibility::Private
    } else {
        PlaylistVisibility::Public
    };
    let update_mode = if req.destructive {
        PlaylistUpdateMode::Destructive
    } else {
        PlaylistUpdateMode::Additive
    };

    match state
        .spotify
        .update_playlist_details(
            &token,
            &row.provider_playlist_id,
            Some(&req.name),
            Some(!req.privacy),
        )
        .await
    {
        Ok(()) => {
            update_playlist_config(
                &state.db.pool,
                playlist_id,
                UpdatePlaylistParams {
                    name: &req.name,
                    visibility,
                    update_mode,
                    update_cadence_days: cadence,
                },
            )
            .await?;

            metrics::counter!("playlist_updated", "service" => "spotify").increment(1);

            Ok(Json(UpdatePlaylistResponse {
                playlist_id: playlist_id.to_string(),
                name: req.name,
                visibility,
                update_mode,
                update_cadence_days: cadence,
            }))
        }
        Err(err) if is_gone_from_spotify(&err) => {
            deactivate_playlist(&state.db.pool, playlist_id).await?;
            Err(AppError::PlaylistUnavailable { playlist_id })
        }
        // Spotify is the source of truth: reject the whole edit, persist nothing.
        Err(err) => Err(err),
    }
}

// ---------------------------------------------------------------------------
// DELETE /playlist/{id}
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct DeletePlaylistResponse {
    pub playlist_id: String,
}

pub async fn delete_playlist(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Path(playlist_id): Path<uuid::Uuid>,
) -> Result<Json<DeletePlaylistResponse>, AppError> {
    let account_id = resolve_account_from_cookie(&state, &jar).await?;

    let row = get_playlist_for_account(&state.db.pool, playlist_id, account_id)
        .await?
        .ok_or(AppError::PlaylistNotFound)?;

    if row.is_active {
        let token = crate::spotify::token::get_valid_spotify_token(&state, account_id).await?;
        match state
            .spotify
            .unfollow_playlist(&token, &row.provider_playlist_id)
            .await
        {
            Ok(()) => {}
            Err(err) if is_gone_from_spotify(&err) => {
                // Already gone — treat as success and proceed to soft-delete.
            }
            Err(err) => return Err(err),
        }
    }
    // If already inactive, there's nothing left to unfollow — skip straight
    // to soft-delete.

    soft_delete_playlist(&state.db.pool, playlist_id).await?;

    Ok(Json(DeletePlaylistResponse {
        playlist_id: playlist_id.to_string(),
    }))
}
