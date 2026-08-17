use super::db::{get_apple_music_account, resolve_account_from_cookie};
use crate::accounts::db::{
    CreatePlaylistParams, create_playlist_record, get_playlist_for_account, soft_delete_playlist,
    update_playlist_cadence,
};
use crate::error::AppError;
use crate::services::playlist_builder::{
    PlaylistUpdateMode, PlaylistVisibility, build_bulleted_description, find_artist_events_near,
    search_tracks_for_artists_apple, validate_cadence_days,
};
use crate::state::AppState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use axum_extra::extract::cookie::SignedCookieJar;
use serde::{Deserialize, Serialize};

/// Every Apple Music playlist is stored `Additive`/`Private` regardless of
/// what a request asks for — the public Apple Music API has no equivalent
/// of Spotify's destructive-replace or public/private toggle for library
/// playlists (see the Phase 0 plan's "Apple API ceiling" note). These are
/// fixed rather than user-configurable.
const APPLE_UPDATE_MODE: PlaylistUpdateMode = PlaylistUpdateMode::Additive;
const APPLE_VISIBILITY: PlaylistVisibility = PlaylistVisibility::Private;

async fn require_apple_music_client(
    state: &AppState,
) -> Result<&crate::apple_music::client::AppleMusicClient, AppError> {
    state
        .apple_music_client
        .as_ref()
        .ok_or_else(|| AppError::Internal("Apple Music client not configured".into()))
}

async fn developer_token(state: &AppState) -> Result<String, AppError> {
    state
        .apple_dev_token
        .as_ref()
        .ok_or_else(|| {
            AppError::Internal("Apple Music developer token manager not configured".into())
        })?
        .get_token()
        .await
}

// ---------------------------------------------------------------------------
// POST /apple/playlist
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct CreatePlaylistRequest {
    pub name: String,
    pub description: Option<String>,
    pub tracks_per_artist: Option<u8>,
    pub cadence: Option<u8>,
    pub location: String,
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(Serialize)]
pub struct CreatePlaylistResponse {
    pub playlist_id: String,
    pub track_count: usize,
    pub tracks: Vec<PlaylistTrack>,
}

#[derive(Serialize)]
pub struct PlaylistTrack {
    pub name: String,
    pub artist: String,
    pub song_id: String,
}

pub async fn create_playlist(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Json(req): Json<CreatePlaylistRequest>,
) -> Result<(StatusCode, Json<CreatePlaylistResponse>), AppError> {
    let account_id = resolve_account_from_cookie(&state, &jar).await?;
    let account = get_apple_music_account(&state.db.pool, account_id)
        .await?
        .ok_or_else(|| AppError::AuthRequired("Apple Music not connected.".into()))?;

    let am = require_apple_music_client(&state).await?;
    let dev_token = developer_token(&state).await?;

    let per_artist = req.tracks_per_artist.unwrap_or(3).min(10);

    let artist_events = find_artist_events_near(&state, req.latitude, req.longitude, 25, 7).await?;
    if artist_events.is_empty() {
        return Err(AppError::Internal(
            "No artists found from nearby Ticketmaster events".to_string(),
        ));
    }
    let artist_names: Vec<String> = artist_events.iter().map(|a| a.name.clone()).collect();

    let track_results =
        search_tracks_for_artists_apple(&state, &dev_token, &artist_names, per_artist).await?;
    if track_results.is_empty() {
        return Err(AppError::ArtistNotFound(
            "No Apple Music tracks found for artists in nearby events".to_string(),
        ));
    }

    let song_ids: Vec<String> = track_results.iter().map(|t| t.uri.clone()).collect();
    let playlist_tracks: Vec<PlaylistTrack> = track_results
        .into_iter()
        .map(|t| PlaylistTrack {
            name: t.name,
            artist: t.artist,
            song_id: t.uri,
        })
        .collect();

    // A user-supplied description wins; otherwise auto-generate one from
    // the nearby artists that populated the playlist.
    let description = req
        .description
        .clone()
        .unwrap_or_else(|| build_bulleted_description(&artist_events));

    let playlist = am
        .create_library_playlist(
            &dev_token,
            &account.music_user_token,
            &req.name,
            Some(&description),
        )
        .await?;

    for chunk in song_ids.chunks(100) {
        am.add_tracks_to_playlist(&dev_token, &account.music_user_token, &playlist.id, chunk)
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

    create_playlist_record(
        &state.db.pool,
        CreatePlaylistParams {
            account_id,
            provider_playlist_id: &playlist.id,
            name: &req.name,
            geohash: &geohash_str,
            city: &req.location,
            update_cadence_days: cadence_days,
            visibility: APPLE_VISIBILITY,
            update_mode: APPLE_UPDATE_MODE,
        },
    )
    .await?;

    metrics::counter!("playlist_created", "service" => "apple_music").increment(1);

    Ok((
        StatusCode::CREATED,
        Json(CreatePlaylistResponse {
            playlist_id: playlist.id,
            track_count: playlist_tracks.len(),
            tracks: playlist_tracks,
        }),
    ))
}

// ---------------------------------------------------------------------------
// GET /apple/playlist
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct PlaylistSummary {
    pub playlist_id: String,
    pub id: String,
    pub name: String,
    pub track_count: usize,
    pub city: String,
    pub update_cadence_days: i16,
    pub is_active: bool,
}

pub async fn get_user_playlists(
    State(state): State<AppState>,
    jar: SignedCookieJar,
) -> Result<Json<Vec<PlaylistSummary>>, AppError> {
    let account_id = resolve_account_from_cookie(&state, &jar).await?;

    let rows = sqlx::query!(
        r#"
        select id, provider_playlist_id, city, update_cadence_days, is_active
        from playlist
        where account_id = $1 and deleted_at is null
        "#,
        account_id
    )
    .fetch_all(&state.db.pool)
    .await?;

    if rows.is_empty() {
        return Ok(Json(vec![]));
    }

    let account = get_apple_music_account(&state.db.pool, account_id)
        .await?
        .ok_or_else(|| AppError::AuthRequired("Apple Music not connected.".into()))?;
    let am = require_apple_music_client(&state).await?;
    let dev_token = developer_token(&state).await?;

    let mut summaries = Vec::with_capacity(rows.len());
    for row in rows {
        if !row.is_active {
            summaries.push(PlaylistSummary {
                playlist_id: row.id.to_string(),
                id: row.provider_playlist_id,
                name: String::new(),
                track_count: 0,
                city: row.city,
                update_cadence_days: row.update_cadence_days,
                is_active: false,
            });
            continue;
        }

        match am
            .get_library_playlist(
                &dev_token,
                &account.music_user_token,
                &row.provider_playlist_id,
            )
            .await
        {
            Ok(details) => {
                let track_count = am
                    .get_library_playlist_tracks(
                        &dev_token,
                        &account.music_user_token,
                        &row.provider_playlist_id,
                    )
                    .await
                    .map(|t| t.len())
                    .unwrap_or(0);

                summaries.push(PlaylistSummary {
                    playlist_id: row.id.to_string(),
                    id: row.provider_playlist_id,
                    name: details.attributes.map(|a| a.name).unwrap_or_default(),
                    track_count,
                    city: row.city,
                    update_cadence_days: row.update_cadence_days,
                    is_active: true,
                })
            }
            Err(err) => {
                // Apple Music offers no explicit "gone" signal equivalent to
                // Spotify's 404 — a 404/403 here most likely means the
                // Music User Token has expired (no refresh flow exists) or
                // the playlist was removed on Apple's side. Either way,
                // there's nothing this app can do to recover it
                // automatically, so surface it as needing reconnection
                // rather than silently dropping it from the list.
                tracing::warn!(
                    playlist_id = %row.provider_playlist_id,
                    error = %err,
                    "failed to fetch Apple Music playlist details"
                );
                summaries.push(PlaylistSummary {
                    playlist_id: row.id.to_string(),
                    id: row.provider_playlist_id,
                    name: String::new(),
                    track_count: 0,
                    city: row.city,
                    update_cadence_days: row.update_cadence_days,
                    is_active: false,
                });
            }
        }
    }

    Ok(Json(summaries))
}

// ---------------------------------------------------------------------------
// PATCH /apple/playlist/{id}  — cadence only, see APPLE_UPDATE_MODE note
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct UpdatePlaylistRequest {
    pub cadence: i16,
}

#[derive(Serialize)]
pub struct UpdatePlaylistResponse {
    pub playlist_id: String,
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
        return Err(AppError::PlaylistNotFound);
    }

    update_playlist_cadence(&state.db.pool, playlist_id, cadence).await?;

    metrics::counter!("playlist_updated", "service" => "apple_music").increment(1);

    Ok(Json(UpdatePlaylistResponse {
        playlist_id: playlist_id.to_string(),
        update_cadence_days: cadence,
    }))
}

// ---------------------------------------------------------------------------
// DELETE /apple/playlist/{id}  — "Remove from Gigeo" only, see module note
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct DeletePlaylistResponse {
    pub playlist_id: String,
}

/// Soft-deletes gigeo's record only. There's no Apple Music API to delete
/// the actual library playlist — it stays in the user's library untouched.
/// The frontend's copy for this action says as much.
pub async fn delete_playlist(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Path(playlist_id): Path<uuid::Uuid>,
) -> Result<Json<DeletePlaylistResponse>, AppError> {
    let account_id = resolve_account_from_cookie(&state, &jar).await?;

    get_playlist_for_account(&state.db.pool, playlist_id, account_id)
        .await?
        .ok_or(AppError::PlaylistNotFound)?;

    soft_delete_playlist(&state.db.pool, playlist_id).await?;

    Ok(Json(DeletePlaylistResponse {
        playlist_id: playlist_id.to_string(),
    }))
}
