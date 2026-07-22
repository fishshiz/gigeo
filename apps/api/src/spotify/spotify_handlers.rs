use crate::auth::TokenResponse;
use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::Query as QueryArray;
use axum_extra::extract::cookie::SignedCookieJar;
use serde::{Deserialize, Serialize};
use tracing::info;
use crate::ticketmaster_stream::{EventsQuery};
use crate::services::playlist_builder::{get_concerts_tm_impl};
use std::collections::HashSet;
use chrono::{Utc, Duration};
use crate::cookie::utils::build_session_cookie;
use crate::error::AppError;
use crate::spotify::client::{Artist, Image};
use crate::state::AppState;

#[derive(sqlx::Type, Clone, Copy)]
#[sqlx(type_name = "playlist_visibility", rename_all = "lowercase")]
enum PlaylistVisibility {
    Public,
    Private,
}

#[derive(sqlx::Type, Clone, Copy)]
#[sqlx(type_name = "playlist_update_mode", rename_all = "lowercase")]
enum PlaylistUpdateMode {
    Additive,
    Destructive,
}

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
    State(state): State<AppState>,
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
        let search_results: Vec<Artist> = state
            .spotify
            .search_artist(&token.access_token, artist_name, 1)
            .await?;
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
    /// Number of tracks to include per artist (default 3, max 10).
    pub tracks_per_artist: Option<u8>,
    // How often to update playlist, in days (min 1, max 60).
    pub cadence: Option<u8>,
    // Whether playlist is public or private.
    pub privacy: bool,
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

pub async fn get_user_playlists(
    State(state): State<AppState>,
    jar: SignedCookieJar,
) -> Result<Json<Vec<CreatePlaylistResponse>>, AppError> {
    let cookie = jar.get("spotify_oauth_state").ok_or(AppError::Unauthorized {
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
    let playlists = get_playlists(&state.db.pool, account_id).await?;
    return Ok(playlists);
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
    let cookie = jar.get("spotify_oauth_state").ok_or_else(|| {
        AppError::AuthRequired("No session cookie. Visit /login first.".into())
    })?;
    let session_id =
        uuid::Uuid::parse_str(cookie.value()).map_err(|_| AppError::Unauthorized {
            status: StatusCode::UNAUTHORIZED,
            message: "Invalid session cookie".into(),
        })?;
    let account_id = resolve_session_account(&state.db.pool, session_id)
        .await?
        .ok_or_else(|| {
            AppError::AuthRequired("Session expired or invalid. Visit /login first.".into())
        })?;

    let user_token = state.user_manager.get_token().await?;
    let cc_token = state.cc_manager.get_token().await?;

    let per_artist = req.tracks_per_artist.unwrap_or(3).min(10);

    let now = Utc::now();

    let query = EventsQuery {
        latitude: req.latitude,
        longitude: req.longitude,
        radius: 25,
        start: now.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        end: (now + Duration::days(7)).format("%Y-%m-%dT%H:%M:%SZ").to_string(),
    };

    let events = get_concerts_tm_impl(&state, &query).await?;
info!("query, {}, {}, {}, {}, {}", &query.end, &query.start, &query.longitude, &query.latitude, &query.radius);
    info!("events, {}",events.len().to_string());

    let artist_names: Vec<String> = events
        .iter()
        .flat_map(|event| event.attractions.as_ref().into_iter().flatten())
        .filter_map(|a| a.name.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    if artist_names.is_empty() {
        return Err(AppError::Internal(
            "No artists found from nearby Ticketmaster events".to_string(),
        ));
    }

    let mut track_uris: Vec<String> = Vec::new();
    let mut playlist_tracks: Vec<PlaylistTrack> = Vec::new();

    for artist_name in &artist_names {
        let tracks = state
            .spotify
            .search_tracks_by_artist(&cc_token.access_token, artist_name, per_artist)
            .await?;

        if let Some(tracks) = tracks {
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
    }

    if track_uris.is_empty() {
        return Err(AppError::ArtistNotFound(
            "No Spotify tracks found for artists in nearby events".to_string(),
        ));
    }

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
            update_mode: PlaylistUpdateMode::Additive,
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
    db: &sqlx::PgPool,
    account_id: uuid::Uuid,
) -> Result<Json<Vec<CreatePlaylistResponse>>, AppError> {
    // Query the database for playlists associated with the user_id
    let playlists = sqlx::query!(
        r#"
        SELECT provider_playlist_id, city
        FROM playlist
        WHERE account_id = $1
        "#,
        account_id
    )
    .fetch_all(db)
    .await?;

    let mut playlist_responses = Vec::new();

    for playlist in playlists {
        // Here you would typically call Spotify API to get more details about the playlist
        // For simplicity, we will just return the playlist ID for now
        playlist_responses.push(CreatePlaylistResponse {
            playlist_id: playlist.provider_playlist_id,
            playlist_url: None, // You would fetch this from Spotify API
            track_count: 0,     // You would fetch this from Spotify API
            tracks: vec![],     // You would fetch this from Spotify API
        });
    }

    Ok(Json(playlist_responses))
}

// ---------------------------------------------------------------------------
// OAuth callback handlers
// ---------------------------------------------------------------------------

/// `GET /login` — redirects the user to Spotify's authorization page.
pub async fn login(State(state): State<AppState>, jar: SignedCookieJar) -> impl IntoResponse {
    let state_param = generate_state();
    let url = state.user_manager.authorize_url(&state_param);

    (jar, Redirect::to(&url))
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
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Query(query): Query<CallbackQuery>,
) -> Result<impl IntoResponse, AppError> {
    tracing::info!(
        "callback hit: code={:?}, state={:?}, error={:?}",
        query.code,
        query.state,
        query.error
    );
    if let Some(err) = query.error {
        return Err(AppError::SpotifyApi {
            status: StatusCode::BAD_REQUEST,
            message: format!("Spotify error: {}", err),
        });
    }
    let token = state
        .user_manager
        .exchange_code(query.code.unwrap_or_default().as_str())
        .await?;

    let me = state
        .user_manager
        .get_current_user(&token.access_token)
        .await?;
    let account_id = upsert_spotify_account(&state.db.pool, &token, &me.id).await?;
    let session_id = create_session(&state.db.pool, account_id).await?;

    let jar = jar.add(build_session_cookie(&state, session_id));

    Ok((jar, Redirect::to("/")))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn generate_state() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..16)
        .map(|_| format!("{:02x}", rng.r#gen::<u8>()))
        .collect()
}

#[derive(Debug, Serialize)]
pub struct AuthStatusResponse {
    logged_in: bool,
    spotify_connected: bool,
    spotify_user_id: Option<String>,
}
pub async fn auth_status(
    State(state): State<AppState>,
    jar: SignedCookieJar,
) -> Result<Json<AuthStatusResponse>, AppError> {
    let cookie = match jar.get("spotify_oauth_state") {
        Some(c) => c,
        None => {
            return Ok(Json(AuthStatusResponse {
                logged_in: false,
                spotify_connected: false,
                spotify_user_id: None,
            }));
        }
    };

    let session_id = uuid::Uuid::parse_str(cookie.value()).map_err(|_| AppError::Unauthorized {
        status: StatusCode::UNAUTHORIZED,
        message: "Unauthorized".into(),
    })?;

    let Some(account_id) = resolve_session_account(&state.db.pool, session_id).await? else {
        return Ok(Json(AuthStatusResponse {
            logged_in: false,
            spotify_connected: false,
            spotify_user_id: None,
        }));
    };

    let spotify = get_spotify_account(&state.db.pool, account_id).await?;

    let resp = match spotify {
        Some(s) => AuthStatusResponse {
            logged_in: true,
            spotify_connected: true,
            spotify_user_id: Some(s.spotify_user_id.clone()),
        },
        None => AuthStatusResponse {
            logged_in: true,
            spotify_connected: false,
            spotify_user_id: None,
        },
    };

    Ok(Json(resp))
}

async fn create_session(
    db: &sqlx::PgPool,
    account_id: uuid::Uuid,
) -> Result<uuid::Uuid, sqlx::Error> {
    let session_id = uuid::Uuid::new_v4();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);

    sqlx::query!(
        r#"
        insert into user_session (id, expires_at, account_id)
        values ($1, $2, $3)
        "#,
        session_id,
        expires_at,
        account_id
    )
    .execute(db)
    .await?;

    Ok(session_id)
}

struct CreatePlaylistParams<'a> {
    account_id: uuid::Uuid,
    provider_playlist_id: &'a str,
    name: &'a str,
    geohash: &'a str,           // must be exactly 6 chars — see check constraint
    city: &'a str,
    update_cadence_days: i16,   // must be 7, 30, or 60 — see check constraint
    visibility: PlaylistVisibility,
    update_mode: PlaylistUpdateMode,
}

async fn create_playlist_record(
    db: &sqlx::PgPool,
    params: CreatePlaylistParams<'_>,
) -> Result<uuid::Uuid, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        insert into playlist (
            account_id, provider_playlist_id, name, geohash, city,
            update_cadence_days, visibility, update_mode
        )
        values ($1, $2, $3, $4, $5, $6, $7, $8)
        returning id
        "#,
        params.account_id,
        params.provider_playlist_id,
        params.name,
        params.geohash,
        params.city,
        params.update_cadence_days,
        params.visibility as _,
        params.update_mode as _,
    )
    .fetch_one(db)
    .await?;

    Ok(row.id)
}

async fn resolve_session_account(
    db: &sqlx::PgPool,
    session_id: uuid::Uuid,
) -> Result<Option<uuid::Uuid>, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        select account_id
        from user_session
        where id = $1
          and revoked_at is null
          and expires_at > now()
        "#,
        session_id
    )
    .fetch_optional(db)
    .await?;

    Ok(row.map(|r| r.account_id))
}

async fn upsert_spotify_account(
    db: &sqlx::PgPool,
    token: &TokenResponse,
    spotify_user_id: &str,
) -> Result<uuid::Uuid, sqlx::Error> {
    let scopes: Vec<String> = token
        .scope
        .clone()
        .unwrap_or_default()
        .split(' ')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();

    let expires_at = chrono::Utc::now() + chrono::Duration::seconds(token.expires_in as i64);

    let mut tx = db.begin().await?;

    let existing = sqlx::query!(
        r#"select account_id from spotify_account where spotify_user_id = $1"#,
        spotify_user_id
    )
    .fetch_optional(&mut *tx)
    .await?;

    let account_id = match existing {
        Some(row) => row.account_id,
        None => {
            let acc = sqlx::query!(
                r#"insert into account (provider) values ('spotify') returning id"#
            )
            .fetch_one(&mut *tx)
            .await?;
            acc.id
        }
    };

    sqlx::query!(
        r#"
        insert into spotify_account (
            account_id, spotify_user_id, access_token, refresh_token,
            token_type, expires_at, scope
        )
        values ($1, $2, $3, $4, $5, $6, $7)
        on conflict (account_id) do update set
            access_token = excluded.access_token,
            refresh_token = excluded.refresh_token,
            token_type = excluded.token_type,
            expires_at = excluded.expires_at,
            scope = excluded.scope,
            updated_at = now()
        "#,
        account_id,
        spotify_user_id,
        token.access_token,
        token.refresh_token.clone().unwrap_or_default(),
        token.token_type,
        expires_at,
        scopes.join(" "),
    )
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(account_id)
}

#[derive(sqlx::FromRow, Debug, Clone)]
struct SpotifyAccountRow {
    account_id: uuid::Uuid,
    spotify_user_id: String,
    access_token: String,
    refresh_token: String,
    scope: String,
    token_type: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

async fn get_spotify_account(
    db: &sqlx::PgPool,
    account_id: uuid::Uuid,
) -> Result<Option<SpotifyAccountRow>, sqlx::Error> {
    sqlx::query_as!(
        SpotifyAccountRow,
        r#"
        select
            account_id,
            spotify_user_id,
            access_token,
            refresh_token,
            scope,
            token_type,
            expires_at
        from spotify_account
        where account_id = $1
        "#,
        account_id
    )
    .fetch_optional(db)
    .await
}
