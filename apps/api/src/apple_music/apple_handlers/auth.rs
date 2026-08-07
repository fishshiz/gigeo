//! Apple Music "connect" flow. Unlike Spotify's redirect-based OAuth, the
//! user token here is obtained client-side via MusicKit JS's
//! `authorize()` — this module just persists what the frontend hands back.

use super::db::{
    APPLE_MUSIC_COOKIE, get_apple_music_account, resolve_account_from_cookie,
    upsert_apple_music_account,
};
use crate::accounts::db::{create_session, revoke_session};
use crate::cookie::utils::{build_expired_cookie, build_session_cookie};
use crate::error::AppError;
use crate::state::AppState;
use axum::{
    Json,
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Redirect},
};
use axum_extra::extract::cookie::SignedCookieJar;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// GET /apple/developer-token
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct DeveloperTokenResponse {
    developer_token: String,
}

/// The frontend needs this to call `MusicKit.configure({ developerToken })`
/// before it can run `authorize()`. Safe to expose publicly — a developer
/// token authenticates *gigeo* to Apple, not an end user; it carries no
/// user data.
pub async fn developer_token(
    State(state): State<AppState>,
) -> Result<Json<DeveloperTokenResponse>, AppError> {
    let manager = state.apple_dev_token.as_ref().ok_or_else(|| {
        AppError::Internal("Apple Music developer token manager not configured".into())
    })?;
    let developer_token = manager.get_token().await?;
    Ok(Json(DeveloperTokenResponse { developer_token }))
}

// ---------------------------------------------------------------------------
// POST /apple/connect
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
pub struct ConnectRequest {
    /// Persistent client-generated UUID (`localStorage`) standing in for
    /// Apple's lack of a stable user ID — see `apple_handlers::db`.
    pub device_id: String,
    pub music_user_token: String,
    pub storefront: String,
}

/// Persists the Music User Token obtained via MusicKit JS on the frontend
/// and starts a session, mirroring Spotify's `oauth_callback`.
pub async fn connect(
    State(state): State<AppState>,
    jar: SignedCookieJar,
    Json(req): Json<ConnectRequest>,
) -> Result<impl IntoResponse, AppError> {
    let account_id = upsert_apple_music_account(
        &state.db.pool,
        &req.device_id,
        &req.music_user_token,
        &req.storefront,
    )
    .await?;
    let session_id = create_session(&state.db.pool, account_id).await?;

    let jar = jar.add(build_session_cookie(&state, session_id, APPLE_MUSIC_COOKIE));

    Ok((jar, StatusCode::NO_CONTENT))
}

// ---------------------------------------------------------------------------
// POST /apple/logout
// ---------------------------------------------------------------------------

pub async fn logout(
    State(state): State<AppState>,
    jar: SignedCookieJar,
) -> Result<impl IntoResponse, AppError> {
    if let Some(session_id) = jar
        .get(APPLE_MUSIC_COOKIE)
        .and_then(|c| uuid::Uuid::parse_str(c.value()).ok())
    {
        revoke_session(&state.db.pool, session_id).await?;
    }

    let jar = jar.add(build_expired_cookie(&state, APPLE_MUSIC_COOKIE));
    Ok((jar, Redirect::to("/")))
}

// ---------------------------------------------------------------------------
// GET /apple/auth-status
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct AppleAuthStatusResponse {
    logged_in: bool,
    apple_music_connected: bool,
}

pub async fn auth_status(
    State(state): State<AppState>,
    jar: SignedCookieJar,
) -> Result<Json<AppleAuthStatusResponse>, AppError> {
    let Ok(account_id) = resolve_account_from_cookie(&state, &jar).await else {
        return Ok(Json(AppleAuthStatusResponse {
            logged_in: false,
            apple_music_connected: false,
        }));
    };

    let account = get_apple_music_account(&state.db.pool, account_id).await?;

    Ok(Json(AppleAuthStatusResponse {
        logged_in: true,
        apple_music_connected: account.is_some(),
    }))
}
