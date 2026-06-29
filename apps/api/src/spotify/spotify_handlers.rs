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

use crate::cookie::utils::build_session_cookie;
use crate::error::AppError;
use crate::spotify::client::{Artist, Image};
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
    State(state): State<AppState>,
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
    create_playlist_record(&state.db.pool, playlist.owner.id, &playlist.id).await?;

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

    let user_id = create_user_if_needed(&state.db.pool).await?;
    upsert_spotify_account(&state.db.pool, user_id, &token).await?;
    let session_id = create_session(&state.db.pool, user_id).await?;

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

    let Some(user_id) = resolve_session_user(&state.db.pool, session_id).await? else {
        return Ok(Json(AuthStatusResponse {
            logged_in: false,
            spotify_connected: false,
            spotify_user_id: None,
        }));
    };

    let spotify = get_spotify_account(&state.db.pool, user_id).await?;

    let resp = match spotify {
        Some(s) => AuthStatusResponse {
            logged_in: true,
            spotify_connected: true,
            spotify_user_id: Some(s.user_id.to_string()),
        },
        None => AuthStatusResponse {
            logged_in: true,
            spotify_connected: false,
            spotify_user_id: None,
        },
    };

    Ok(Json(resp))
}

async fn create_session(db: &sqlx::PgPool, user_id: uuid::Uuid) -> Result<uuid::Uuid, sqlx::Error> {
    let session_id = uuid::Uuid::new_v4();
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);

    sqlx::query!(
        r#"
        insert into user_session (id, user_id, expires_at)
        values ($1, $2, $3)
        "#,
        session_id,
        user_id,
        expires_at
    )
    .execute(db)
    .await?;

    Ok(session_id)
}

async fn create_user_if_needed(db: &sqlx::PgPool) -> Result<uuid::Uuid, sqlx::Error> {
    let id = uuid::Uuid::new_v4();
    sqlx::query!("insert into app_user (id) values ($1)", id)
        .execute(db)
        .await?;
    Ok(id)
}

async fn create_playlist_record(
    db: &sqlx::PgPool,
    spotify_user_id: String,
    spotify_playlist_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        insert into spotify_playlist (spotify_account_user_id, spotify_playlist_id)
        values ($1, $2)
        "#,
        spotify_user_id,
        spotify_playlist_id
    )
    .execute(db)
    .await?;
    Ok(())
}

async fn resolve_session_user(
    db: &sqlx::PgPool,
    session_id: uuid::Uuid,
) -> Result<Option<uuid::Uuid>, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        select user_id
        from user_session
        where id = $1
          and revoked_at is null
          and expires_at > now()
        "#,
        session_id
    )
    .fetch_optional(db)
    .await?;

    Ok(row.map(|r| r.user_id))
}

#[derive(sqlx::FromRow, Debug, Clone)]
struct SpotifyAccountRow {
    user_id: uuid::Uuid,
    access_token: String,
    refresh_token: String,
    scope: String,
    spotify_user_id: String,
    token_type: String,
    expires_at: chrono::DateTime<chrono::Utc>,
}

async fn upsert_spotify_account(
    db: &sqlx::PgPool,
    user_id: uuid::Uuid,
    token: &TokenResponse,
) -> Result<(), sqlx::Error> {
    let scopes: Vec<String> = token
        .scope
        .clone()
        .unwrap_or_default()
        .split(' ')
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .collect();

    let expires_at = chrono::Utc::now() + std::time::Duration::from_secs(token.expires_in);

    sqlx::query!(
        r#"
        insert into spotify_account (
            user_id, access_token, refresh_token, token_type, expires_at
        )
        values ($1, $2, $3, $4, $5)
        on conflict (user_id) do update set
            access_token = excluded.access_token,
            refresh_token = excluded.refresh_token,
            token_type = excluded.token_type,
            expires_at = excluded.expires_at,
            updated_at = now()
        "#,
        user_id,
        token.access_token,
        token.refresh_token.clone().unwrap_or_default(),
        token.token_type,
        expires_at,
    )
    .execute(db)
    .await?;

    Ok(())
}

async fn get_spotify_account(
    db: &sqlx::PgPool,
    user_id: uuid::Uuid,
) -> Result<Option<SpotifyAccountRow>, sqlx::Error> {
    sqlx::query_as!(
        SpotifyAccountRow,
        r#"
        select
            user_id,
            access_token,
            refresh_token,
            scope,
            spotify_user_id,
            token_type,
            expires_at
        from spotify_account
        where user_id = $1
        "#,
        user_id
    )
    .fetch_optional(db)
    .await
}
